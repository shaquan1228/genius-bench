# frozen_string_literal: true

RSpec.describe "sum type variants" do
  variants = {
    GeniusBench::Value => %w[Null Bool Int Float String List Record],
    GeniusBench::Predicate => %w[Present Equals Within Contains Words All Any Not],
    GeniusBench::Verification => %w[Unchecked Passed Failed InvalidSubmission TimedOut HarnessError],
    GeniusBench::Assessment => %w[Check All],
    GeniusBench::Criterion => %w[Code Answer State]
  }

  variants.each do |base, names|
    describe base.name do
      it "nests exactly one subclass per core variant" do
        nested = base.constants(false).map { |name| base.const_get(name) }.select { |constant| constant.is_a?(Class) && constant < base }
        expect(nested.map { |constant| constant.name.delete_prefix("#{base.name}::") }).to match_array(names)
      end

      it "cannot be instantiated without a variant" do
        expect { base.allocate }.to raise_error(TypeError)
      end

      it "drops the flattened accessors" do
        expect(base.public_instance_methods(false)).not_to include(:kind, :scalar, :detail, :path, :checks, :predicate)
      end
    end
  end

  let(:leaf) { GeniusBench::Predicate::Present.new(path: ["a"]) }

  it "narrows values with is_a? and case/in" do
    converted = GeniusBench::Value.from_ruby(value: {"n" => nil, "b" => true, "i" => 1, "f" => 1.5, "s" => "x", "l" => [1], "r" => {}})
    expect(converted).to be_a(GeniusBench::Value::Record)
    seen = converted.fields.to_h do |name, item|
      plain = case item
      in GeniusBench::Value::Null then nil
      in GeniusBench::Value::Bool(value:) then value
      in GeniusBench::Value::Int(value:) then value
      in GeniusBench::Value::Float(value:) then value
      in GeniusBench::Value::String(value:) then value
      in GeniusBench::Value::List(items:) then items.map(&:value)
      in GeniusBench::Value::Record(fields:) then fields.to_h
      end
      [name, plain]
    end
    expect(seen).to eq("n" => nil, "b" => true, "i" => 1, "f" => 1.5, "s" => "x", "l" => [1], "r" => {})
  end

  it "builds every value variant from its own fields" do
    built = GeniusBench::Value::Record.new(
      fields: [
        ["list", GeniusBench::Value::List.new(items: [GeniusBench::Value::Int.new(value: 1), 2])],
        ["text", GeniusBench::Value::String.new(value: "x")],
        ["null", GeniusBench::Value::Null.new],
        ["flag", GeniusBench::Value::Bool.new(value: false)],
        ["ratio", GeniusBench::Value::Float.new(value: 0.5)]
      ]
    )
    expect(built).to be_equal_to(other: {"list" => [1, 2], "text" => "x", "null" => nil, "flag" => false, "ratio" => 0.5})
    expect(GeniusBench::Record.new(fields: [["x", 1]]).to_kernel_value).to be_a(GeniusBench::Value::Record)
  end

  it "narrows predicates with is_a? and case/in" do
    built = [
      leaf,
      GeniusBench::Predicate::Equals.new(path: ["a"], expected: {"x" => 1}),
      GeniusBench::Predicate::Within.new(path: ["a"], allowed: [1, "x"]),
      GeniusBench::Predicate::Contains.new(path: ["a"], item: 1),
      GeniusBench::Predicate::Words.new(path: ["a"], min_words: 1, max_words: 2),
      GeniusBench::Predicate::All.new(checks: [["leaf", leaf]]),
      GeniusBench::Predicate::Any.new(checks: [["leaf", leaf]]),
      GeniusBench::Predicate::Not.new(check: leaf)
    ]
    summaries = built.map do |predicate|
      expect(predicate).to be_a(GeniusBench::Predicate).and be_frozen
      case predicate
      in GeniusBench::Predicate::Present(path:) then [:present, path]
      in GeniusBench::Predicate::Equals(path:, expected:) then [:equals, path, expected.get(path: ["x"]).value]
      in GeniusBench::Predicate::Within(path:, allowed:) then [:within, path, allowed.size]
      in GeniusBench::Predicate::Contains(path:, item:) then [:contains, path, item.value]
      in GeniusBench::Predicate::Words(path:, min_words:, max_words:) then [:words, path, min_words, max_words]
      in GeniusBench::Predicate::All(checks:) then [:all, checks.map(&:first)]
      in GeniusBench::Predicate::Any(checks:) then [:any, checks.map(&:first)]
      in GeniusBench::Predicate::Not(check:) then [:not, check.class]
      end
    end
    expect(summaries).to eq(
      [
        [:present, ["a"]], [:equals, ["a"], 1], [:within, ["a"], 2], [:contains, ["a"], 1],
        [:words, ["a"], 1, 2], [:all, ["leaf"]], [:any, ["leaf"]], [:not, GeniusBench::Predicate::Present]
      ]
    )
  end

  it "narrows verifications and keeps them through outcomes" do
    built = [
      GeniusBench::Verification::Unchecked.new, GeniusBench::Verification::Passed.new,
      GeniusBench::Verification::Failed.new, GeniusBench::Verification::InvalidSubmission.new,
      GeniusBench::Verification::TimedOut.new, GeniusBench::Verification::HarnessError.new(detail: "worker missing")
    ]
    expect(built.map(&:status)).to eq(%w[unchecked passed failed invalid_submission timed_out harness_error])
    built.each do |verification|
      returned = build_outcome(verification: verification).verification
      expect(returned).to be_an_instance_of(verification.class).and eq(verification)
      case returned
      in GeniusBench::Verification::HarnessError(detail:) then expect(detail).to eq("worker missing")
      in GeniusBench::Verification then expect(returned).not_to respond_to(:detail)
      end
    end
  end

  it "narrows assessments" do
    check = GeniusBench::Predicate::Equals.new(path: ["answer"], expected: 42)
    case GeniusBench::Assessment.new(predicate: check, observation: {"answer" => 42})
    in GeniusBench::Assessment::Check(predicate:, observation:)
      expect(predicate).to be_a(GeniusBench::Predicate::Equals)
      expect(observation.get(path: ["answer"]).value).to eq(42)
    end
    combined = GeniusBench::Assessment.new(predicate: GeniusBench::Predicate::All.new(checks: [["answer", check]]), observation: {"answer" => 41})
    expect(combined).to be_a(GeniusBench::Assessment::All)
    expect(combined.checks.map { |name, nested| [name, nested.class] }).to eq([["answer", GeniusBench::Assessment::Check]])
    rebuilt = GeniusBench::Assessment::All.new(checks: [["answer", GeniusBench::Assessment::Check.new(predicate: check, observation: 42)]])
    expect(rebuilt.scores).to eq([["answer", false]])
  end

  it "narrows criteria and keeps them through tasks" do
    verifier = GeniusBench::Verifier.new(inputs: ["a.txt"], files: [], tools: [], command: ["true"], stdin: "", timeout_seconds: 1)
    rubric = GeniusBench::Rubric.new(expected: [["answer", 42]], text_rule: nil)
    [
      GeniusBench::Criterion::Code.new(verifier: verifier),
      GeniusBench::Criterion::Answer.new(rubric: rubric),
      GeniusBench::Criterion::State.new(predicate: leaf)
    ].each do |criterion|
      declared = GeniusBench::Task.new(
        id: "variant",
        fixture: [GeniusBench::SourceFile.new(path: "a.txt", contents: "")],
        instruction: GeniusBench::Instruction.new(prompt: "observe"),
        criterion: criterion,
        allowed_files: ["a.txt"],
        reference: GeniusBench::Reference.new(edits: [], completion: {"answer" => 42})
      ).criterion
      expect(declared).to be_an_instance_of(criterion.class)
      case declared
      in GeniusBench::Criterion::Code(verifier:) then expect(verifier.command).to eq(["true"])
      in GeniusBench::Criterion::Answer(rubric:) then expect(rubric.expected.first.first).to eq("answer")
      in GeniusBench::Criterion::State(predicate:) then expect(predicate).to be_a(GeniusBench::Predicate::Present)
      end
    end
  end

  it "takes keyword arguments only", contract: true do
    expect { GeniusBench::Verification::HarnessError.new("lost") }.to raise_error(ArgumentError)
    expect { GeniusBench::Predicate::Equals.new(["a"], 1) }.to raise_error(ArgumentError)
    expect { GeniusBench::Verification::Passed.new(detail: "x") }.to raise_error(ArgumentError)
  end
end
