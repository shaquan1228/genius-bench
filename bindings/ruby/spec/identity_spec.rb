# frozen_string_literal: true

RSpec.describe "value identity" do
  file = -> { GeniusBench::SourceFile.new(path: "a.txt", contents: "data") }
  tool = -> { GeniusBench::SourceFile.new(path: "tool", contents: "public") }
  verifier = -> { GeniusBench::Verifier.new(inputs: ["a.txt"], files: [], tools: [tool.call], command: ["check"], stdin: "", timeout_seconds: 1) }
  rubric = -> { GeniusBench::Rubric.new(expected: [["answer", GeniusBench::Value.from_ruby(value: 42)]], text_rule: nil) }
  task = lambda do
    GeniusBench::Task.new(
      id: "code",
      fixture: [file.call],
      instruction: GeniusBench::Instruction.new(prompt: "change"),
      criterion: GeniusBench::Criterion.code(verifier: verifier.call),
      allowed_files: ["a.txt"],
      reference: GeniusBench::Reference.new(edits: [], completion: GeniusBench::Value.from_ruby(value: nil))
    )
  end
  outcome = lambda do
    GeniusBench::Outcome.new(
      changed_files: [], verification: GeniusBench::Verification.passed, response: GeniusBench::Value.from_ruby(value: 1), facts: {}
    )
  end
  invalid = lambda do
    GeniusBench::Task.new(
      id: "bad id",
      fixture: [],
      instruction: GeniusBench::Instruction.new(prompt: "p"),
      criterion: GeniusBench::Criterion.state(predicate: GeniusBench::Predicate.present(path: [])),
      allowed_files: [],
      reference: GeniusBench::Reference.new(edits: [], completion: GeniusBench::Value.from_ruby(value: nil))
    )
  rescue GeniusBench::ValidationError => error
    error.issues[0]
  end

  {
    GeniusBench::Value => -> { GeniusBench::Value.from_ruby(value: {"x" => [1, 2.5]}) },
    GeniusBench::Record => -> { GeniusBench::Record.new(fields: [["x", 1], ["x", 2]]) },
    GeniusBench::Predicate => -> { GeniusBench::Predicate.words(path: ["t"], min_words: 1, max_words: 2) },
    GeniusBench::Verification => -> { GeniusBench::Verification.harness_error(detail: "lost") },
    GeniusBench::Outcome => outcome,
    GeniusBench::Assessment => -> { GeniusBench::Assessment.new(predicate: GeniusBench::Predicate.present(path: []), observation: GeniusBench::Value.from_ruby(value: 1)) },
    GeniusBench::SourceFile => file,
    GeniusBench::Instruction => -> { GeniusBench::Instruction.new(prompt: "change") },
    GeniusBench::Verifier => verifier,
    GeniusBench::TextRule => -> { GeniusBench::TextRule.new(field: "reason", min_words: 1, max_words: 5) },
    GeniusBench::Rubric => rubric,
    GeniusBench::Criterion => -> { GeniusBench::Criterion.answer(rubric: rubric.call) },
    GeniusBench::Reference => -> { GeniusBench::Reference.new(edits: [file.call], completion: GeniusBench::Value.from_ruby(value: "done")) },
    GeniusBench::Case => -> { task.call.project },
    GeniusBench::Issue => invalid,
    GeniusBench::Task => task
  }.each do |klass, build|
    describe klass do
      subject(:object) { build.call }

      let(:same) { build.call }

      describe "#==" do
        it { is_expected.to eq(same) }
        it { is_expected.not_to equal(same) }
        it { is_expected.not_to eq(Object.new) }
      end

      describe "#eql?" do
        it { is_expected.to eql(same) }
      end

      describe "#hash" do
        it "agrees with ==" do
          expect(object.hash).to eq(same.hash)
        end
      end

      describe "#inspect" do
        it "names the class" do
          expect(object.inspect).to start_with("#<#{klass} ")
        end
      end
    end
  end

  describe GeniusBench::Verification do
    it "distinguishes statuses" do
      expect(described_class.passed).not_to eq(described_class.failed)
    end
  end
end
