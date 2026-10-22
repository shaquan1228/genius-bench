let quote value =
  let buffer = Buffer.create (String.length value + 2) in
  Buffer.add_char buffer '"';
  String.iter
    (function
      | '"' -> Buffer.add_string buffer "\\\""
      | '\\' -> Buffer.add_string buffer "\\\\"
      | c when Char.code c < 32 -> Buffer.add_string buffer (Printf.sprintf "\\u%04x" (Char.code c))
      | c -> Buffer.add_char buffer c)
    value;
  Buffer.add_char buffer '"';
  Buffer.contents buffer

let array render values = "[" ^ String.concat "," (List.map render values) ^ "]"
let object_ fields = "{" ^ String.concat "," (List.map (fun (key, value) -> quote key ^ ":" ^ value) fields) ^ "}"
let pair render (name, value) = array Fun.id [ quote name; render value ]
let strings = array quote
let boolean = string_of_bool
let tagged kind fields = object_ (("type", quote kind) :: fields)

let rec value = function
  | Value.Null -> tagged "null" []
  | Value.Bool { value = item } -> tagged "bool" [ ("value", boolean item) ]
  | Value.Int { value = item } -> tagged "int" [ ("value", quote (string_of_int item)) ]
  | Value.Float { value = item } -> tagged "float" [ ("value", quote (Printf.sprintf "%.17g" item)) ]
  | Value.String { value = item } -> tagged "string" [ ("value", quote item) ]
  | Value.List { items } -> tagged "list" [ ("items", array value items) ]
  | Value.Record { fields } -> tagged "record" [ ("fields", array (pair value) fields) ]

let optional render = function None -> "null" | Some item -> render item

let rec predicate = function
  | Predicate.Present { path } -> tagged "present" [ ("path", strings path) ]
  | Predicate.Equals { path; expected } -> tagged "equals" [ ("path", strings path); ("expected", value expected) ]
  | Predicate.Within { path; allowed } -> tagged "within" [ ("path", strings path); ("allowed", array value allowed) ]
  | Predicate.Contains { path; item } -> tagged "contains" [ ("path", strings path); ("item", value item) ]
  | Predicate.Words { path; min_words; max_words } -> tagged "words" [ ("path", strings path); ("min_words", string_of_int min_words); ("max_words", string_of_int max_words) ]
  | Predicate.All { checks } -> tagged "all" [ ("checks", array (pair predicate) checks) ]
  | Predicate.Any { checks } -> tagged "any" [ ("checks", array (pair predicate) checks) ]
  | Predicate.Not { check } -> tagged "not" [ ("check", predicate check) ]

let verification item =
  match item with
  | Outcome.Harness_error { detail } -> tagged "harness_error" [ ("detail", quote detail) ]
  | _ -> tagged (Outcome.status ~verification:item) []

let outcome (item : Outcome.t) = object_ [ ("changed_files", strings item.changed_files); ("verification", verification item.verification); ("response", value item.response); ("facts", array (pair value) item.facts) ]
let file (item : Task.file) = object_ [ ("path", quote item.path); ("contents", quote item.contents) ]
let instruction (item : Task.instruction) = object_ [ ("skill", quote item.skill); ("prompt", quote item.prompt) ]
let verifier (item : Task.verifier) = object_ [ ("inputs", strings item.inputs); ("files", array file item.files); ("tools", array file item.tools); ("command", strings item.command); ("stdin", quote item.stdin); ("timeout_seconds", string_of_int item.timeout_seconds) ]
let text_rule (item : Task.text_rule) = object_ [ ("field", quote item.field); ("min_words", string_of_int item.min_words); ("max_words", string_of_int item.max_words) ]
let rubric (item : Task.rubric) = object_ [ ("expected", array (pair value) item.expected); ("text_rule", optional text_rule item.text_rule) ]
let criterion = function
  | Task.Code { verifier = item } -> tagged "code" [ ("verifier", verifier item) ]
  | Task.Answer { rubric = item } -> tagged "answer" [ ("rubric", rubric item) ]
  | Task.State { predicate = item } -> tagged "state" [ ("predicate", predicate item) ]
let reference (item : Task.reference) = object_ [ ("edits", array file item.edits); ("completion", value item.completion) ]
let task (item : Task.t) = object_ [ ("id", quote item.id); ("fixture", array file item.fixture); ("instruction", instruction item.instruction); ("criterion", criterion item.criterion); ("allowed_files", strings item.allowed_files); ("reference", reference item.reference) ]
let case (item : Task.case) = object_ [ ("case_id", quote item.case_id); ("skill", quote item.skill); ("prompt", quote item.prompt); ("files", array file item.files) ]
let scores = array (pair boolean)

let rec assessment item =
  let detail = match item with
    | Assessment.Check { predicate = check; observation } -> tagged "check" [ ("predicate", predicate check); ("observation", value observation) ]
    | Assessment.All { checks } -> tagged "all" [ ("checks", array (pair assessment) checks) ]
  in
  object_ [ ("detail", detail); ("passed", boolean (Assessment.passed ~assessment:item)); ("observed", optional value (Assessment.observed ~assessment:item)); ("scores", scores (Assessment.scores ~assessment:item)) ]

let validation (item : Failure.validation_failure) = object_ [ ("case_id", optional quote item.case_id); ("issues", array (fun (issue : Failure.issue) -> object_ [ ("field", quote issue.field); ("message", quote issue.message) ]) item.issues) ]
let grading = function Failure.Verification_harness_error { detail } -> tagged "verification_harness_error" [ ("detail", quote detail) ]
let rows = ref []
let emit category id input expected = rows := object_ [ ("category", quote category); ("id", quote id); ("input", input); ("expected", expected) ] :: !rows
let s value = Value.String { value }
let i value = Value.Int { value }
let f value = Value.Float { value }
let r fields = Value.Record { fields }
let l items = Value.List { items }
let file_ path contents : Task.file = { path; contents }
let out ?(changed_files = []) ?(response = Value.Null) ?(facts = []) verification = Outcome.create ~changed_files ~response ~facts ~verification ()
let ok = Result.get_ok
let checks : Task.verifier = { inputs = [ "source.txt" ]; files = [ file_ "verifier.py" "private checker" ]; tools = [ file_ "helper.txt" "public helper" ]; command = [ "checker" ]; stdin = "private input"; timeout_seconds = 5 }
let code : Task.t = { id = "clear-task"; fixture = [ file_ "source.txt" "seed"; file_ "visible.txt" "public check" ]; instruction = { skill = "reasoning"; prompt = "Change source.txt only." }; criterion = Task.Code { verifier = checks }; allowed_files = [ "source.txt" ]; reference = { edits = [ file_ "source.txt" "fixed" ]; completion = s "fixed" } }
let construct (item : Task.t) = Task.create ~id:item.id ~fixture:item.fixture ~instruction:item.instruction ~criterion:item.criterion ~allowed_files:item.allowed_files ~reference:item.reference
let answer expected text_rule : Task.t = { code with id = "answer"; fixture = []; criterion = Task.Answer { rubric = { expected; text_rule } }; allowed_files = []; reference = { edits = []; completion = Value.Null } }
let decision = answer [ ("choice", s "B") ] (Some { Task.field = "reason"; min_words = 1; max_words = 40 })
let call name = r [ ("name", s name); ("arguments", r [ ("id", i 7) ]) ]
let state : Task.t = { (answer [] None) with id = "tool-task"; criterion = Task.State { predicate = Predicate.All { checks = [ ("looked-up", Predicate.Contains { path = [ "trace" ]; item = call "lookup" }); ("only-lookups", Predicate.Within { path = [ "trace" ]; allowed = [ call "lookup" ] }); ("balance", Predicate.Equals { path = [ "state"; "balance" ]; expected = i 5 }); ("no-refund", Predicate.Not { check = Predicate.Present { path = [ "state"; "refund" ] } }); ("answered", Predicate.Equals { path = [ "response"; "choice" ]; expected = s "B" }) ] } } }
let emit_task id item =
  let expected = match construct item with
    | Error failure -> object_ [ ("failure", validation failure) ]
    | Ok task_ -> object_ [ ("case", case (Task.project ~task:task_)); ("checks", optional verifier (Task.checks ~task:task_)); ("condition", predicate (Task.condition ~criterion:task_.criterion)) ]
  in emit "task" id (task item) expected
let emit_grade id task_ outcome_ =
  let task_ = ok (construct task_) in
  let expected = match Task.assess ~task:task_ ~outcome:outcome_ with
    | Error failure -> object_ [ ("failure", grading failure) ]
    | Ok result -> object_ [ ("assessment", assessment result); ("scores", scores (ok (Task.grade ~task:task_ ~outcome:outcome_))); ("observation", value (Outcome.observation ~outcome:outcome_)) ]
  in emit "grade" id (object_ [ ("task", task task_); ("outcome", outcome outcome_) ]) expected

let () =
  List.iter (fun (id, left, right) -> emit "equal" id (object_ [ ("left", value left); ("right", value right) ]) (boolean (Value.equal ~left ~right)))
    [ ("record-order", r [ ("a", i 1); ("b", i 2) ], r [ ("b", i 2); ("a", i 1) ]);
      ("first-duplicate", r [ ("a", i 1); ("a", i 2) ], r [ ("a", i 1) ]);
      ("different-first-duplicate", r [ ("a", i 2); ("a", i 1) ], r [ ("a", i 1) ]);
      ("extra-key", r [ ("a", i 1) ], r [ ("a", i 1); ("b", i 2) ]);
      ("nested-record-order", l [ r [ ("a", i 1); ("b", i 2) ] ], l [ r [ ("b", i 2); ("a", i 1) ] ]);
      ("list-order", l [ i 1; i 2 ], l [ i 2; i 1 ]);
      ("int-float", i 1, f 1.0); ("bool-int", Value.Bool { value = true }, i 1);
      ("bool-float", Value.Bool { value = true }, f 1.0); ("null", Value.Null, Value.Null);
      ("empty-list", l [], l []); ("empty-record", r [], r []); ("empty-alternatives", l [], r []);
      ("nan", f nan, f nan); ("positive-infinity", f infinity, f infinity); ("negative-infinity", f neg_infinity, f neg_infinity);
      ("signed-zero", f 0.0, f (-0.0)); ("max-int", i max_int, i max_int); ("min-int", i min_int, i min_int) ];
  let observation = r [ ("response", r [ ("choice", s "B"); ("reason", s "one two three") ]); ("trace", l [ call "lookup" ]); ("state", r [ ("balance", i 5); ("note", Value.Null) ]); ("empty", l []); ("dup", i 1); ("dup", i 2) ] in
  List.iter (fun (id, path) -> emit "get" id (object_ [ ("value", value observation); ("path", strings path) ]) (optional value (Value.get ~value:observation ~path)))
    [ ("root", []); ("nested", [ "state"; "balance" ]); ("missing", [ "state"; "missing" ]); ("null", [ "state"; "note" ]); ("scalar-path", [ "response"; "choice"; "deeper" ]); ("duplicate", [ "dup" ]); ("list-path", [ "trace"; "0" ]) ];
  let yes = Predicate.Present { path = [ "state" ] } and no = Predicate.Present { path = [ "absent" ] } in
  let checks_ = [
    ("present", yes); ("present-null", Predicate.Present { path = [ "state"; "note" ] }); ("present-missing", no);
    ("equals", Predicate.Equals { path = [ "response"; "choice" ]; expected = s "B" }); ("equals-wrong", Predicate.Equals { path = [ "response"; "choice" ]; expected = s "A" }); ("equals-null", Predicate.Equals { path = [ "state"; "note" ]; expected = Value.Null }); ("equals-missing-null", Predicate.Equals { path = [ "absent" ]; expected = Value.Null });
    ("contains", Predicate.Contains { path = [ "trace" ]; item = call "lookup" }); ("contains-missing", Predicate.Contains { path = [ "trace" ]; item = s "lookup" }); ("contains-wrong-type", Predicate.Contains { path = [ "state" ]; item = call "lookup" });
    ("within", Predicate.Within { path = [ "trace" ]; allowed = [ call "lookup" ] }); ("within-forbidden", Predicate.Within { path = [ "trace" ]; allowed = [] }); ("within-wrong-type", Predicate.Within { path = [ "response" ]; allowed = [] }); ("within-empty", Predicate.Within { path = [ "empty" ]; allowed = [] });
    ("words-inclusive", Predicate.Words { path = [ "response"; "reason" ]; min_words = 3; max_words = 3 }); ("words-too-short", Predicate.Words { path = [ "response"; "reason" ]; min_words = 4; max_words = 9 }); ("words-wrong-type", Predicate.Words { path = [ "state"; "balance" ]; min_words = 0; max_words = 9 });
    ("all-empty", Predicate.All { checks = [] }); ("any-empty", Predicate.Any { checks = [] }); ("all", Predicate.All { checks = [ ("yes", yes); ("no", no) ] }); ("any", Predicate.Any { checks = [ ("yes", yes); ("no", no) ] }); ("not", Predicate.Not { check = no }); ("not-false", Predicate.Not { check = yes });
    ("nested-all", Predicate.All { checks = [ ("first", yes); ("nested", Predicate.All { checks = [ ("duplicate", no); ("duplicate", yes) ] }) ] }) ] in
  List.iter (fun (id, check) -> emit "predicate" id (object_ [ ("predicate", predicate check); ("observation", value observation) ]) (object_ [ ("holds", boolean (Predicate.holds ~predicate:check ~observation)); ("assessment", assessment (Assessment.create ~predicate:check ~observation)) ])) checks_;
  let separators = List.init 5 (( + ) 9) @ List.init 5 (( + ) 28) @ [ 133; 160; 5760 ] @ List.init 11 (( + ) 8192) @ [ 8232; 8233; 8239; 8287; 12288; 0x200B; 0xFEFF ] in
  let utf8 code = let buffer = Buffer.create 4 in Buffer.add_utf_8_uchar buffer (Uchar.of_int code); Buffer.contents buffer in
  List.iter (fun code -> let text = "first" ^ utf8 code ^ "second" in let check = Predicate.Words { path = []; min_words = 2; max_words = 2 } in emit "predicate" (Printf.sprintf "separator-%04x" code) (object_ [ ("predicate", predicate check); ("observation", value (s text)) ]) (object_ [ ("holds", boolean (Predicate.holds ~predicate:check ~observation:(s text))); ("assessment", assessment (Assessment.create ~predicate:check ~observation:(s text))) ])) separators;
  List.iter (fun (id, item) -> emit_task id item) [ ("code", code); ("answer", decision); ("state", state); ("zero-words", answer [] (Some { Task.field = "note"; min_words = 0; max_words = 0 })); ("invalid-id", { code with id = "" }); ("invalid-name", { code with id = "Upper_name" }); ("invalid-skill", { code with instruction = { code.instruction with skill = "bad_skill" } }); ("outside-reference", { code with allowed_files = [] }); ("missing-input", { code with fixture = [ file_ "other.txt" "seed" ] }); ("artifact-overlap", { code with fixture = file_ "verifier.py" "tampered" :: code.fixture }); ("invalid-path", { code with fixture = file_ "../bad" "x" :: code.fixture }); ("duplicate-fixture", { code with fixture = code.fixture @ code.fixture }); ("duplicate-edits", { code with reference = { code.reference with edits = code.reference.edits @ code.reference.edits } }); ("empty-rubric", answer [] None); ("text-collision", answer [ ("choice", s "B") ] (Some { Task.field = "choice"; min_words = 0; max_words = 2 })); ("inverted-text", answer [] (Some { Task.field = "reason"; min_words = 3; max_words = 2 })); ("negative-text", answer [] (Some { Task.field = "reason"; min_words = -1; max_words = 2 })); ("empty-prompt", { code with instruction = { code.instruction with prompt = "" } }); ("duplicate-allowed", { code with allowed_files = [ "source.txt"; "source.txt" ] }); ("duplicate-expectations", answer [ ("choice", s "B"); ("choice", s "B") ] None); ("empty-text-field", answer [] (Some { Task.field = ""; min_words = 0; max_words = 1 })) ];
  List.iter (fun (id, verifier_) -> emit_task id { code with criterion = Task.Code { verifier = verifier_ } })
    [ ("empty-inputs", { checks with inputs = [] }); ("duplicate-inputs", { checks with inputs = [ "source.txt"; "source.txt" ] }); ("empty-command", { checks with command = [] }); ("empty-command-part", { checks with command = [ "checker"; "" ] }); ("zero-timeout", { checks with timeout_seconds = 0 }); ("negative-timeout", { checks with timeout_seconds = -1 }); ("invalid-artifact", { checks with tools = [ file_ "/secret" "x" ] }); ("overlapping-input-artifact", { checks with files = [ file_ "source.txt" "x" ] }); ("duplicate-artifacts", { checks with tools = checks.files }); ("all-issues", { inputs = []; files = [ file_ "../bad" "x"; file_ "../bad" "x" ]; tools = []; command = [ "" ]; stdin = ""; timeout_seconds = 0 }) ];
  emit_task "combined-issues" { id = ""; instruction = { skill = ""; prompt = "" }; fixture = [ file_ "../bad" "x"; file_ "../bad" "x" ]; allowed_files = []; reference = { code.reference with edits = [ file_ "outside.txt" "x"; file_ "outside.txt" "x" ] }; criterion = Task.Code { verifier = { checks with inputs = []; files = [ file_ "../bad" "x"; file_ "../bad" "x" ]; command = [ "" ]; timeout_seconds = 0 } } };
  let statuses = [ Outcome.Unchecked; Outcome.Passed; Outcome.Failed; Outcome.Invalid_submission; Outcome.Timed_out; Outcome.Harness_error { detail = "offline" } ] in
  List.iteri (fun index changed_files -> List.iter (fun status -> emit_grade (Printf.sprintf "code-%d-%s" index (Outcome.status ~verification:status)) code (out ~changed_files status)) statuses) [ []; [ "source.txt" ]; [ "other.txt" ]; [ "../outside" ]; [ "source.txt"; "source.txt" ] ];
  List.iter (fun (id, response) -> emit_grade id decision (out ~response Outcome.Unchecked)) [ ("answer-correct", r [ ("choice", s "B"); ("reason", s "Cheaper") ]); ("answer-reordered-extra", r [ ("reason", s "Cheaper"); ("choice", s "B"); ("extra", Value.Bool { value = true }) ]); ("answer-empty", r []); ("answer-wrong", r [ ("choice", s "A"); ("reason", s "Cheaper") ]); ("answer-null", Value.Null); ("answer-wrong-type", r [ ("choice", s "B"); ("reason", Value.Bool { value = true }) ]); ("answer-first-duplicate", r [ ("choice", s "B"); ("choice", s "A"); ("reason", s "one") ]) ];
  List.iter (fun count -> emit_grade (Printf.sprintf "answer-words-%d" count) decision (out ~response:(r [ ("choice", s "B"); ("reason", s (String.concat " " (List.init count (fun _ -> "word")))) ]) Outcome.Unchecked)) [ 0; 1; 40; 41 ];
  List.iter (fun (id, trace, state_, response) -> emit_grade id state (out ~response ~facts:[ ("trace", l trace); ("state", state_) ] Outcome.Unchecked)) [ ("state-correct", [ call "lookup" ], r [ ("balance", i 5) ], r [ ("choice", s "B") ]); ("state-no-trace", [], r [ ("balance", i 5) ], r [ ("choice", s "B") ]); ("state-extra-call", [ call "lookup"; call "refund" ], r [ ("balance", i 5) ], r [ ("choice", s "B") ]); ("state-null", [ call "lookup" ], Value.Null, r [ ("choice", s "B") ]); ("state-no-response", [ call "lookup" ], r [ ("balance", i 5) ], Value.Null); ("state-refund", [ call "lookup" ], r [ ("balance", i 5); ("refund", i 1) ], r [ ("choice", s "B") ]) ];
  emit_grade "reserved-facts" code (out ~changed_files:[ "other.txt" ] ~facts:[ ("verification", s "passed"); ("changed_files", l []); ("response", s "forged"); ("trace", l []); ("trace", l [ call "refund" ]) ] Outcome.Failed);
  emit_grade "answer-harness-failure" decision (out (Outcome.Harness_error { detail = "answer offline" }));
  emit_grade "state-harness-failure" state (out (Outcome.Harness_error { detail = "state offline" }));
  print_endline (array Fun.id (List.rev !rows))
