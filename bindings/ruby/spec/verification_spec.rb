# frozen_string_literal: true

RSpec.describe GeniusBench::Verification do
  {
    "Unchecked" => "unchecked", "Passed" => "passed", "Failed" => "failed",
    "InvalidSubmission" => "invalid_submission", "TimedOut" => "timed_out"
  }.each do |name, status|
    describe "::#{name}" do
      subject { described_class.const_get(name).new }

      it { is_expected.to have_attributes(status: status) }
      it { is_expected.to be_frozen }
      it { is_expected.not_to respond_to(:detail) }
    end
  end

  describe "::HarnessError" do
    subject { described_class::HarnessError.new(detail: "worker missing") }

    it { is_expected.to have_attributes(status: "harness_error", detail: "worker missing") }

    context "with a non-string detail", contract: true do
      it "raises TypeError" do
        expect { described_class::HarnessError.new(detail: 1) }.to raise_error(TypeError)
      end
    end
  end

  describe "#status" do
    subject { described_class::Passed.new.status }

    it { is_expected.to be_frozen }
  end
end
