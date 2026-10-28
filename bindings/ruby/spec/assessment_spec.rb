# frozen_string_literal: true

RSpec.describe GeniusBench::Assessment do
  let(:rubric) do
    GeniusBench::Rubric.new(
      expected: [["answer", kernel(42)]],
      text_rule: GeniusBench::TextRule.new(field: "reason", min_words: 1, max_words: 5)
    )
  end
  let(:condition) { GeniusBench::Criterion::Answer.new(rubric: rubric).condition }
  let(:observation) { kernel({"response" => {"answer" => 42, "reason" => "six sevens"}}) }

  describe ".new" do
    subject(:assessment) { described_class.new(predicate: condition, observation: observation) }

    it { is_expected.to be_a(GeniusBench::Assessment::All).and not_respond_to(:predicate, :observation) }
    it { is_expected.to be_passed }

    it "scores every named check" do
      expect(assessment.scores).to eq([["response", true], ["answer", true], ["text", true]])
    end

    it "returns frozen scores" do
      expect(assessment.scores).to be_frozen
    end

    it "exposes nested checks" do
      expect(assessment.checks[1][1]).to be_a(GeniusBench::Assessment::Check)
    end

    it "retains the observed value behind a check" do
      expect(assessment.checks[1][1].observed.value).to eq(42)
    end

    it "retains the predicate behind a check" do
      expect(assessment.checks[1][1].predicate).to be_a(GeniusBench::Predicate::Equals)
    end

    context "with a native observation" do
      subject(:assessment) { described_class.new(predicate: condition, observation: {"response" => {"answer" => 42, "reason" => "six sevens"}}) }

      it { is_expected.to be_passed }
    end

    context "with a single predicate" do
      subject(:assessment) { described_class.new(predicate: GeniusBench::Predicate::Present.new(path: ["response"]), observation: observation) }

      it { is_expected.to be_a(GeniusBench::Assessment::Check).and not_respond_to(:checks) }

      it "scores as passed" do
        expect(assessment.scores).to eq([["passed", true]])
      end

      it "exposes the whole observation" do
        expect(assessment.observation).to be_a(GeniusBench::Value::Record)
      end
    end

    context "with a failing rubric" do
      let(:observation) { kernel({"response" => {"answer" => 41, "reason" => "six sevens"}}) }

      it { is_expected.not_to be_passed }

      it "scores the failing check" do
        expect(assessment.scores).to include(["answer", false])
      end
    end
  end
end
