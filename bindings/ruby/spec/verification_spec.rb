# frozen_string_literal: true

RSpec.describe GeniusBench::Verification do
  %w[unchecked passed failed invalid_submission timed_out].each do |name|
    describe ".#{name}" do
      subject { described_class.public_send(name) }

      it { is_expected.to have_attributes(status: name, detail: nil) }
      it { is_expected.to be_frozen }
    end
  end

  describe ".harness_error" do
    subject { described_class.harness_error(detail: "worker missing") }

    it { is_expected.to have_attributes(status: "harness_error", detail: "worker missing") }

    context "with a non-string detail", contract: true do
      it "raises TypeError" do
        expect { described_class.harness_error(detail: 1) }.to raise_error(TypeError)
      end
    end
  end

  describe "#status" do
    subject { described_class.passed.status }

    it { is_expected.to be_frozen }
  end
end
