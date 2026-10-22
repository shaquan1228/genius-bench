# frozen_string_literal: true

RSpec.describe GeniusBench::Predicate do
  let(:observation) { kernel({ "present" => 1, "null" => nil, "list" => [1, "x"], "nested" => { "k" => nil }, "t" => text }) }
  let(:text) { "one two" }

  describe ".present" do
    subject(:predicate) { described_class.present(path: ["present"]) }

    it { is_expected.to have_attributes(kind: "present", path: ["present"]) }
    it("holds") { is_expected.to be_holds(observation: observation) }

    it "returns a frozen path" do
      expect(predicate.path).to be_frozen
    end

    context "when the value is null" do
      subject { described_class.present(path: ["null"]) }

      it("does not hold") { is_expected.not_to be_holds(observation: observation) }
    end

    context "when the path is missing" do
      subject { described_class.present(path: ["missing"]) }

      it("does not hold") { is_expected.not_to be_holds(observation: observation) }
    end

    context "with a non-string path element" do
      it "raises TypeError" do
        expect { described_class.present(path: [1]) }.to raise_error(TypeError)
      end
    end

    context "with a path that is not an array" do
      it "raises TypeError" do
        expect { described_class.present(path: "a") }.to raise_error(TypeError)
      end
    end

    context "with a path element that only converts through to_str" do
      let(:convertible) { Object.new.tap { |object| def object.to_str = "a" } }

      it "raises TypeError" do
        expect { described_class.present(path: [convertible]) }.to raise_error(TypeError)
      end
    end
  end

  describe ".equals" do
    subject(:predicate) { described_class.equals(path: ["null"], expected: kernel(nil)) }

    it { is_expected.to have_attributes(kind: "equals") }
    it("holds") { is_expected.to be_holds(observation: observation) }

    it "exposes the expected value" do
      expect(predicate.expected.kind).to eq("null")
    end

    context "when the path is missing" do
      subject { described_class.equals(path: ["missing"], expected: kernel(nil)) }

      it("does not hold") { is_expected.not_to be_holds(observation: observation) }
    end
  end

  describe ".within" do
    subject(:predicate) { described_class.within(path: ["list"], allowed: [kernel(1), kernel("x")]) }

    it { is_expected.to have_attributes(kind: "within", check: nil, checks: []) }
    it("holds") { is_expected.to be_holds(observation: observation) }

    it "exposes the allowed values" do
      expect(predicate.allowed.map(&:scalar)).to eq([1, "x"])
    end
  end

  describe ".contains" do
    subject(:predicate) { described_class.contains(path: ["list"], item: kernel("x")) }

    it { is_expected.to have_attributes(kind: "contains") }
    it("holds") { is_expected.to be_holds(observation: observation) }

    it "exposes the item" do
      expect(predicate.item.scalar).to eq("x")
    end

    context "when the item differs by numeric type" do
      subject { described_class.contains(path: ["list"], item: kernel(1.0)) }

      it("does not hold") { is_expected.not_to be_holds(observation: observation) }
    end
  end

  describe ".words" do
    subject(:predicate) { described_class.words(path: ["t"], min_words: 2, max_words: 2) }

    it { is_expected.to have_attributes(kind: "words", min_words: 2, max_words: 2) }
    it("holds") { is_expected.to be_holds(observation: observation) }

    context "when the count is below the minimum" do
      subject { described_class.words(path: ["t"], min_words: 3, max_words: 4) }

      it("does not hold") { is_expected.not_to be_holds(observation: observation) }
    end

    context "when the count is above the maximum" do
      subject { described_class.words(path: ["t"], min_words: 1, max_words: 1) }

      it("does not hold") { is_expected.not_to be_holds(observation: observation) }
    end

    context "with empty text" do
      subject { described_class.words(path: ["t"], min_words: 0, max_words: 0) }

      let(:text) { "" }

      it("holds") { is_expected.to be_holds(observation: observation) }
    end

    context "with the original whitespace set" do
      subject { described_class.words(path: ["t"], min_words: 4, max_words: 4) }

      let(:text) { "a b　c\u0085d" }

      it("holds") { is_expected.to be_holds(observation: observation) }
    end

    context "with characters outside the whitespace set" do
      subject { described_class.words(path: ["t"], min_words: 1, max_words: 1) }

      let(:text) { "a​b᠎c" }

      it("holds") { is_expected.to be_holds(observation: observation) }
    end

    context "with a large bound" do
      subject { described_class.words(path: [], min_words: 1, max_words: 2**40) }

      it { is_expected.to have_attributes(max_words: 2**40) }
    end

    context "with non-integer bounds" do
      [1.5, Rational(3, 2), true, "2", nil].each do |invalid|
        it "raises TypeError for #{invalid.inspect}" do
          expect { described_class.words(path: [], min_words: invalid, max_words: 2) }.to raise_error(TypeError)
        end
      end
    end

    context "with a bound that only converts through to_int" do
      let(:convertible) { Object.new.tap { |object| def object.to_int = 1 } }

      it "raises TypeError" do
        expect { described_class.words(path: [], min_words: convertible, max_words: 1) }.to raise_error(TypeError)
      end
    end
  end

  describe ".all" do
    subject(:predicate) { described_class.all(checks: checks) }

    let(:checks) { [["one", described_class.present(path: ["present"])], ["two", described_class.present(path: ["null"])]] }

    it { is_expected.to have_attributes(kind: "all", path: nil) }
    it("does not hold") { is_expected.not_to be_holds(observation: observation) }

    it "exposes the named checks" do
      expect(predicate.checks.map(&:first)).to eq(%w[one two])
    end

    context "without checks" do
      let(:checks) { [] }

      it("holds") { is_expected.to be_holds(observation: observation) }
    end

    context "with many checks" do
      let(:checks) { Array.new(300) { |index| [index.to_s, described_class.present(path: [])] } }

      it "keeps every check" do
        expect(predicate.checks.size).to eq(300)
      end
    end

    context "with a check that is not a predicate" do
      let(:checks) { [["one", 1]] }

      it "raises TypeError" do
        expect { predicate }.to raise_error(TypeError)
      end
    end
  end

  describe ".any" do
    subject(:predicate) { described_class.any(checks: checks) }

    let(:checks) { [["one", described_class.present(path: ["null"])], ["two", described_class.present(path: ["present"])]] }

    it { is_expected.to have_attributes(kind: "any") }
    it("holds") { is_expected.to be_holds(observation: observation) }

    context "without checks" do
      let(:checks) { [] }

      it("does not hold") { is_expected.not_to be_holds(observation: observation) }
    end
  end

  describe ".not" do
    subject(:predicate) { described_class.not(check: described_class.present(path: %w[nested k])) }

    it { is_expected.to have_attributes(kind: "not") }
    it("holds") { is_expected.to be_holds(observation: observation) }

    it "exposes the negated check" do
      expect(predicate.check.kind).to eq("present")
    end
  end

  %w[not all any].each do |operator|
    describe "nesting through .#{operator}" do
      let(:wrap) do
        lambda do |inner|
          operator == "not" ? described_class.not(check: inner) : described_class.public_send(operator, checks: [["x", inner]])
        end
      end
      let(:deepest) { (1..255).reduce(described_class.present(path: [])) { |inner, _| wrap.call(inner) } }

      it "accepts 256 levels" do
        expect(GeniusBench::Assessment.new(predicate: deepest, observation: kernel(1)).passed?).to eq(operator != "not")
      end

      it "rejects 257 levels" do
        expect { wrap.call(deepest) }.to raise_error(ArgumentError, /nesting/)
      end
    end
  end

  describe "#holds?" do
    it "requires the observation keyword" do
      expect { described_class.present(path: []).holds?(observation) }.to raise_error(ArgumentError)
    end
  end
end
