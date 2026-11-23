# frozen_string_literal: true

require "genius_bench"

Order = Data.define(:total_cents) do
  def to_kernel_value
    {"total_cents" => total_cents, "note" => nil}
  end
end

def verify_order_adapter
  order = Order.new(total_cents: 4200)
  check = GeniusBench::Predicate::Equals.new(path: ["total_cents"], expected: 4200)
  raise "Order did not match the predicate" unless check.holds?(observation: order)
  raise "Wrong order total matched" if check.holds?(observation: Order.new(total_cents: 4199))
  raise "Float matched the integer total" if check.holds?(observation: {"total_cents" => 4200.0})
  case check
  in GeniusBench::Predicate::Equals(path:, expected: GeniusBench::Value::Int(value: cents))
    raise "Unexpected predicate fields" unless [path, cents] == [["total_cents"], 4200]
  else
    raise "Expected an integer equality predicate"
  end

  value = GeniusBench::Value.from_ruby(value: order)
  raise "Missing field returned a value" unless value.get(path: ["missing"]).nil?
  raise "Note did not return a kernel null" unless value.get(path: ["note"]).is_a?(GeniusBench::Value::Null)

  original = GeniusBench::Record.new(fields: [["total_cents", 4200], ["note", nil]])
  reordered = GeniusBench::Record.new(fields: [["note", nil], ["total_cents", 4200], ["total_cents", 4199]])
  raise "Equivalent records compared unequal" unless original == reordered && original.eql?(reordered)
  raise "Equivalent records hashed differently" unless original.hash == reordered.hash
  raise "Equivalent records did not share a Hash key" unless {original => "first", reordered => "second"} == {original => "second"}
end

specification = Gem.loaded_specs["genius-bench-kernel"]
raise "Version mismatch" if specification && specification.version.to_s != GeniusBench.version
if ENV["GEM_HOME"]
  gem_path = specification && File.realpath(specification.full_gem_path)
  raise "Loaded outside the gem home" unless gem_path&.start_with?(File.realpath(ENV["GEM_HOME"]))
end
if ENV["KERNEL_SOURCE_HASH"]
  raise "Source identity mismatch" unless GeniusBench.source_hash == ENV["KERNEL_SOURCE_HASH"]
end

verify_order_adapter

task = GeniusBench::Task.new(
  id: "installed",
  fixture: [],
  instruction: GeniusBench::Instruction.new(prompt: "observe"),
  criterion: GeniusBench::Criterion::State.new(predicate: GeniusBench::Predicate::Present.new(path: ["response"])),
  allowed_files: [],
  reference: GeniusBench::Reference.new(edits: [], completion: true)
)
outcome = GeniusBench::Outcome.new(
  changed_files: [],
  verification: GeniusBench::Verification::Unchecked.new,
  response: GeniusBench::Value.from_ruby(value: true),
  facts: []
)
raise "Unexpected scores" unless task.grade(outcome: outcome) == [["correctness", true], ["scope", true]]
raise "Unexpected reference scores" unless task.grade_reference(verification: GeniusBench::Verification::Unchecked.new) == [["correctness", true], ["scope", true]]

puts RUBY_VERSION, $LOADED_FEATURES.grep(/semantic_ruby/).first, GeniusBench.version, GeniusBench.source_hash
