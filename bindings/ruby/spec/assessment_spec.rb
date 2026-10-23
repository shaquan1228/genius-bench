# frozen_string_literal: true

RSpec.describe GeniusBench::Assessment do
  let(:rubric) do
    GeniusBench::Rubric.new(
      expected: [["answer", kernel(42)]],
      text_rule: GeniusBench::TextRule.new(field: "reason", min_words: 1, max_words: 5)
    )
  end
  let(:condition) { GeniusBench::Criterion.answer(rubric: rubric).condition }
  let(:observation) { kernel({ "response" => { "answer" => 42, "reason" => "six sevens" } }) }

  describe ".new" do
    subject(:assessment) { described_class.new(predicate: condition, observation: observation) }

    it { is_expected.to have_attributes(kind: "all", predicate: nil, observation: nil) }
    it { is_expected.to be_passed }

    it "scores every named check" do
      expect(assessment.scores).to eq([["response", true], ["answer", true], ["text", true]])
    end

    it "returns frozen scores" do
      expect(assessment.scores).to be_frozen
    end

    it "exposes nested checks" do
      expect(assessment.checks[1][1]).to have_attributes(kind: "check")
    end

    it "retains the observed value behind a check" do
      expect(assessment.checks[1][1].observed.scalar).to eq(42)
    end

    it "retains the predicate behind a check" do
      expect(assessment.checks[1][1].predicate.kind).to eq("equals")
    end

    context "with a single predicate" do
      subject(:assessment) { described_class.new(predicate: GeniusBench::Predicate.present(path: ["response"]), observation: observation) }

      it { is_expected.to have_attributes(kind: "check", checks: []) }

      it "scores as passed" do
        expect(assessment.scores).to eq([["passed", true]])
      end

      it "exposes the whole observation" do
        expect(assessment.observation.kind).to eq("record")
      end
    end

    context "with a failing rubric" do
      let(:observation) { kernel({ "response" => { "answer" => 41, "reason" => "six sevens" } }) }

      it { is_expected.not_to be_passed }

      it "scores the failing check" do
        expect(assessment.scores).to include(["answer", false])
      end
    end
  end
end
