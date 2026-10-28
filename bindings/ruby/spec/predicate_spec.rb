# frozen_string_literal: true

RSpec.describe GeniusBench::Predicate do
  let(:observation) { kernel({"present" => 1, "null" => nil, "list" => [1, "x"], "nested" => {"k" => nil}, "t" => text}) }
  let(:text) { "one two" }

  describe "::Present" do
    subject(:predicate) { described_class::Present.new(path: ["present"]) }

    it { is_expected.to be_a(GeniusBench::Predicate::Present).and have_attributes(path: ["present"]) }
    it("holds") { is_expected.to be_holds(observation: observation) }

    it "returns a frozen path" do
      expect(predicate.path).to be_frozen
    end

    context "when the value is null" do
      subject { described_class::Present.new(path: ["null"]) }

      it("does not hold") { is_expected.not_to be_holds(observation: observation) }
    end

    context "when the path is missing" do
      subject { described_class::Present.new(path: ["missing"]) }

      it("does not hold") { is_expected.not_to be_holds(observation: observation) }
    end

    context "with a native observation" do
      it("holds") { is_expected.to be_holds(observation: {"present" => 1}) }
    end

    context "with a non-string path element", contract: true do
      it "raises TypeError" do
        expect { described_class::Present.new(path: [1]) }.to raise_error(TypeError)
      end
    end

    context "with a path that is not an array", contract: true do
      it "raises TypeError" do
        expect { described_class::Present.new(path: "a") }.to raise_error(TypeError)
      end
    end

    context "with a path element that only converts through to_str", contract: true do
      let(:convertible) { Object.new.tap { |object| def object.to_str = "a" } }

      it "raises TypeError" do
        expect { described_class::Present.new(path: [convertible]) }.to raise_error(TypeError)
      end
    end
  end

  describe "::Equals" do
    subject(:predicate) { described_class::Equals.new(path: ["null"], expected: kernel(nil)) }

    it { is_expected.to be_a(GeniusBench::Predicate::Equals) }
    it("holds") { is_expected.to be_holds(observation: observation) }

    it "exposes the expected value" do
      expect(predicate.expected).to be_a(GeniusBench::Value::Null)
    end

    context "when the path is missing" do
      subject { described_class::Equals.new(path: ["missing"], expected: kernel(nil)) }

      it("does not hold") { is_expected.not_to be_holds(observation: observation) }
    end

    context "with a native expected value" do
      subject { described_class::Equals.new(path: ["nested"], expected: {"k" => nil}) }

      it("holds") { is_expected.to be_holds(observation: observation) }
    end
  end

  describe "::Within" do
    subject(:predicate) { described_class::Within.new(path: ["list"], allowed: [kernel(1), kernel("x")]) }

    it { is_expected.to be_a(GeniusBench::Predicate::Within).and not_respond_to(:check, :checks) }
    it("holds") { is_expected.to be_holds(observation: observation) }

    it "exposes the allowed values" do
      expect(predicate.allowed.map(&:value)).to eq([1, "x"])
    end

    context "with native allowed values" do
      subject { described_class::Within.new(path: ["list"], allowed: [1, "x"]) }

      it("holds") { is_expected.to be_holds(observation: observation) }
    end
  end

  describe "::Contains" do
    subject(:predicate) { described_class::Contains.new(path: ["list"], item: kernel("x")) }

    it { is_expected.to be_a(GeniusBench::Predicate::Contains) }
    it("holds") { is_expected.to be_holds(observation: observation) }

    it "exposes the item" do
      expect(predicate.item.value).to eq("x")
    end

    context "when the item differs by numeric type" do
      subject { described_class::Contains.new(path: ["list"], item: kernel(1.0)) }

      it("does not hold") { is_expected.not_to be_holds(observation: observation) }
    end

    context "with a native item" do
      subject { described_class::Contains.new(path: ["list"], item: "x") }

      it("holds") { is_expected.to be_holds(observation: observation) }
    end

    context "with an unsupported item", contract: true do
      it "raises TypeError" do
        expect { described_class::Contains.new(path: ["list"], item: Object.new) }.to raise_error(TypeError)
      end
    end
  end

  describe "::Words" do
    subject(:predicate) { described_class::Words.new(path: ["t"], min_words: 2, max_words: 2) }

    it { is_expected.to be_a(GeniusBench::Predicate::Words).and have_attributes(min_words: 2, max_words: 2) }
    it("holds") { is_expected.to be_holds(observation: observation) }

    context "when the count is below the minimum" do
      subject { described_class::Words.new(path: ["t"], min_words: 3, max_words: 4) }

      it("does not hold") { is_expected.not_to be_holds(observation: observation) }
    end

    context "when the count is above the maximum" do
      subject { described_class::Words.new(path: ["t"], min_words: 1, max_words: 1) }

      it("does not hold") { is_expected.not_to be_holds(observation: observation) }
    end

    context "with empty text" do
      subject { described_class::Words.new(path: ["t"], min_words: 0, max_words: 0) }

      let(:text) { "" }

      it("holds") { is_expected.to be_holds(observation: observation) }
    end

    context "with the original whitespace set" do
      subject { described_class::Words.new(path: ["t"], min_words: 4, max_words: 4) }

      let(:text) { "a b　c\u0085d" }

      it("holds") { is_expected.to be_holds(observation: observation) }
    end

    context "with characters outside the whitespace set" do
      subject { described_class::Words.new(path: ["t"], min_words: 1, max_words: 1) }

      let(:text) { "a​b᠎c" }

      it("holds") { is_expected.to be_holds(observation: observation) }
    end

    context "with a large bound" do
      subject { described_class::Words.new(path: [], min_words: 1, max_words: 2**40) }

      it { is_expected.to have_attributes(max_words: 2**40) }
    end

    context "with non-integer bounds", contract: true do
      [1.5, Rational(3, 2), true, "2", nil].each do |invalid|
        it "raises TypeError for #{invalid.inspect}" do
          expect { described_class::Words.new(path: [], min_words: invalid, max_words: 2) }.to raise_error(TypeError)
        end
      end
    end

    context "with a bound that only converts through to_int", contract: true do
      let(:convertible) { Object.new.tap { |object| def object.to_int = 1 } }

      it "raises TypeError" do
        expect { described_class::Words.new(path: [], min_words: convertible, max_words: 1) }.to raise_error(TypeError)
      end
    end
  end

  describe "::All" do
    subject(:predicate) { described_class::All.new(checks: checks) }

    let(:checks) { [["one", described_class::Present.new(path: ["present"])], ["two", described_class::Present.new(path: ["null"])]] }

    it { is_expected.to be_a(GeniusBench::Predicate::All).and not_respond_to(:path) }
    it("does not hold") { is_expected.not_to be_holds(observation: observation) }

    it "exposes the named checks" do
      expect(predicate.checks.map(&:first)).to eq(%w[one two])
    end

    context "without checks" do
      let(:checks) { [] }

      it("holds") { is_expected.to be_holds(observation: observation) }
    end

    context "with many checks" do
      let(:checks) { Array.new(300) { |index| [index.to_s, described_class::Present.new(path: [])] } }

      it "keeps every check" do
        expect(predicate.checks.size).to eq(300)
      end
    end

    context "with a check that is not a predicate", contract: true do
      let(:checks) { [["one", 1]] }

      it "raises TypeError" do
        expect { predicate }.to raise_error(TypeError)
      end
    end

    context "with a Hash of checks keyed by symbols" do
      let(:checks) { {one: described_class::Present.new(path: ["present"]), two: described_class::Present.new(path: ["null"])} }

      it "names the checks in entry order" do
        expect(predicate.checks.map(&:first)).to eq(%w[one two])
      end
    end
  end

  describe "::Any" do
    subject(:predicate) { described_class::Any.new(checks: checks) }

    let(:checks) { [["one", described_class::Present.new(path: ["null"])], ["two", described_class::Present.new(path: ["present"])]] }

    it { is_expected.to be_a(GeniusBench::Predicate::Any) }
    it("holds") { is_expected.to be_holds(observation: observation) }

    context "without checks" do
      let(:checks) { [] }

      it("does not hold") { is_expected.not_to be_holds(observation: observation) }
    end
  end

  describe "::Not" do
    subject(:predicate) { described_class::Not.new(check: described_class::Present.new(path: %w[nested k])) }

    it { is_expected.to be_a(GeniusBench::Predicate::Not) }
    it("holds") { is_expected.to be_holds(observation: observation) }

    it "exposes the negated check" do
      expect(predicate.check).to be_a(GeniusBench::Predicate::Present)
    end
  end

  %w[Not All Any].each do |operator|
    describe "::#{operator}" do
      let(:wrap) do
        lambda do |inner|
          (operator == "Not") ? described_class::Not.new(check: inner) : described_class.const_get(operator).new(checks: [["x", inner]])
        end
      end
      let(:deepest) { (1..255).reduce(described_class::Present.new(path: [])) { |inner, _| wrap.call(inner) } }

      context "when nested to the limit" do
        it "accepts #{GeniusBench::NESTING_LIMIT} levels" do
          expect(GeniusBench::Assessment.new(predicate: deepest, observation: kernel(1)).passed?).to eq(operator != "Not")
        end

        it "rejects one more level" do
          expect { wrap.call(deepest) }.to raise_error(ArgumentError, /nesting/)
        end
      end
    end
  end

  describe "#holds?" do
    it "requires the observation keyword", contract: true do
      expect { described_class::Present.new(path: []).holds?(observation) }.to raise_error(ArgumentError)
    end
  end
end
