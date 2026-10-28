# frozen_string_literal: true

RSpec.shared_examples "a frozen declaration" do
  it { is_expected.to be_frozen }

  it "rejects new instance variables" do
    expect { subject.instance_variable_set(:@extra, 1) }.to raise_error(FrozenError)
  end
end

RSpec.describe GeniusBench::SourceFile do
  describe ".new" do
    subject(:file) { described_class.new(path: "a.txt", contents: "data") }

    it_behaves_like "a frozen declaration"
    it { is_expected.to have_attributes(path: "a.txt", contents: "data") }

    it "returns frozen strings" do
      expect(file.path).to be_frozen
    end

    context "with a non-string path", contract: true do
      it "raises TypeError" do
        expect { described_class.new(path: 1, contents: "") }.to raise_error(TypeError)
      end
    end
  end
end

RSpec.describe GeniusBench::Instruction do
  describe ".new" do
    subject { described_class.new(prompt: "observe") }

    it_behaves_like "a frozen declaration"
    it { is_expected.to have_attributes(prompt: "observe") }

    context "with a positional prompt", contract: true do
      it "raises ArgumentError" do
        expect { described_class.new("observe") }.to raise_error(ArgumentError)
      end
    end

    context "without a prompt", contract: true do
      it "raises ArgumentError" do
        expect { described_class.new }.to raise_error(ArgumentError)
      end
    end

    context "with a prompt that only converts through to_str", contract: true do
      let(:convertible) { Object.new.tap { |object| def object.to_str = "a" } }

      it "raises TypeError" do
        expect { described_class.new(prompt: convertible) }.to raise_error(TypeError)
      end
    end
  end
end

RSpec.describe GeniusBench::Verifier do
  describe ".new" do
    subject(:verifier) do
      described_class.new(
        inputs: ["a.txt"],
        files: [GeniusBench::SourceFile.new(path: "private", contents: "secret")],
        tools: [GeniusBench::SourceFile.new(path: "tool", contents: "public")],
        command: ["check"],
        stdin: "secret",
        timeout_seconds: 1
      )
    end

    it_behaves_like "a frozen declaration"
    it { is_expected.to have_attributes(inputs: ["a.txt"], command: ["check"], stdin: "secret", timeout_seconds: 1) }

    it "exposes private files" do
      expect(verifier.files.map(&:path)).to eq(["private"])
    end

    it "exposes public tools" do
      expect(verifier.tools.map(&:path)).to eq(["tool"])
    end

    context "with a fractional timeout", contract: true do
      it "raises TypeError" do
        expect { described_class.new(inputs: [], files: [], tools: [], command: [], stdin: "", timeout_seconds: 1.9) }.to raise_error(TypeError)
      end
    end

    context "with a file that is not a SourceFile", contract: true do
      it "raises TypeError" do
        expect { described_class.new(inputs: [], files: ["no"], tools: [], command: [], stdin: "", timeout_seconds: 1) }.to raise_error(TypeError)
      end
    end
  end
end

RSpec.describe GeniusBench::TextRule do
  describe ".new" do
    subject { described_class.new(field: "reason", min_words: 1, max_words: 5) }

    it_behaves_like "a frozen declaration"
    it { is_expected.to have_attributes(field: "reason", min_words: 1, max_words: 5) }

    context "with non-integer bounds", contract: true do
      [1.5, Rational(3, 2), true, "2", nil].each do |invalid|
        it "raises TypeError for #{invalid.inspect}" do
          expect { described_class.new(field: "f", min_words: 0, max_words: invalid) }.to raise_error(TypeError)
        end
      end
    end
  end
end

RSpec.describe GeniusBench::Rubric do
  describe ".new" do
    subject(:rubric) { described_class.new(expected: [["answer", kernel(42)]], text_rule: nil) }

    it_behaves_like "a frozen declaration"
    it { is_expected.to have_attributes(text_rule: nil) }

    it "exposes the expected fields" do
      expect(scalars_of(rubric.expected)).to eq([["answer", 42]])
    end

    context "with a native expected value" do
      subject(:rubric) { described_class.new(expected: {answer: 42}, text_rule: nil) }

      it "converts the expected value" do
        expect(scalars_of(rubric.expected)).to eq([["answer", 42]])
      end
    end

    context "with an unsupported expected value", contract: true do
      it "raises TypeError" do
        expect { described_class.new(expected: [["answer", Object.new]], text_rule: nil) }.to raise_error(TypeError)
      end
    end

    context "with a text rule" do
      subject { described_class.new(expected: [], text_rule: GeniusBench::TextRule.new(field: "reason", min_words: 1, max_words: 5)) }

      it "exposes the rule" do
        expect(subject.text_rule.field).to eq("reason")
      end
    end
  end
end

RSpec.describe GeniusBench::Criterion do
  let(:rubric) do
    described_class::Answer.new(
      rubric: GeniusBench::Rubric.new(
        expected: [["answer", kernel(42)]],
        text_rule: GeniusBench::TextRule.new(field: "reason", min_words: 1, max_words: 5)
      )
    )
  end

  describe "::Answer" do
    subject(:criterion) { rubric }

    it_behaves_like "a frozen declaration"
    it { is_expected.to be_a(GeniusBench::Criterion::Answer).and not_respond_to(:verifier, :predicate) }

    it "exposes the rubric" do
      expect(criterion.rubric.text_rule.field).to eq("reason")
    end

    describe "#condition" do
      subject(:condition) { criterion.condition }

      it { is_expected.to be_a(GeniusBench::Predicate::All) }

      it "checks the response, each field, and the text rule" do
        expect(condition.checks.map(&:first)).to eq(%w[response answer text])
      end

      it "binds the text rule to the field path" do
        expect(condition.checks[2][1].path).to eq(%w[response reason])
      end

      it "binds the text rule bounds" do
        expect(condition.checks[2][1]).to have_attributes(min_words: 1, max_words: 5)
      end
    end
  end

  describe "::Code" do
    subject(:criterion) do
      described_class::Code.new(verifier: GeniusBench::Verifier.new(inputs: ["a"], files: [], tools: [], command: ["check"], stdin: "", timeout_seconds: 1))
    end

    it { is_expected.to be_a(GeniusBench::Criterion::Code).and not_respond_to(:rubric, :predicate) }

    it "conditions on a passed verification" do
      expect(criterion.condition).to be_a(GeniusBench::Predicate::Equals).and have_attributes(path: ["verification"])
    end
  end

  describe "::State" do
    subject(:criterion) { described_class::State.new(predicate: GeniusBench::Predicate::Present.new(path: ["response"])) }

    it { is_expected.to be_a(GeniusBench::Criterion::State).and not_respond_to(:rubric, :verifier) }

    it "conditions on the predicate" do
      expect(criterion.condition).to be_a(GeniusBench::Predicate::Present)
    end
  end
end

RSpec.describe GeniusBench::Reference do
  describe ".new" do
    subject(:reference) { described_class.new(edits: [GeniusBench::SourceFile.new(path: "a.txt", contents: "new")], completion: "done") }

    it_behaves_like "a frozen declaration"

    it "exposes the edits" do
      expect(reference.edits.map(&:path)).to eq(["a.txt"])
    end

    it "exposes the completion" do
      expect(reference.completion.value).to eq("done")
    end

    it "accepts nil as the completion" do
      expect(described_class.new(edits: [], completion: nil).completion).to be_a(GeniusBench::Value::Null)
    end
  end
end
