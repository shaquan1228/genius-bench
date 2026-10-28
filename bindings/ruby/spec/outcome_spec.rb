# frozen_string_literal: true

RSpec.describe GeniusBench::Outcome do
  describe ".new" do
    subject(:outcome) do
      build_outcome(changed_files: ["a.txt"], verification: GeniusBench::Verification::Failed.new, response: kernel(1), facts: facts)
    end

    let(:facts) { [["extra", kernel(true)]] }

    it { is_expected.to be_frozen }

    it "snapshots the changed files" do
      expect(outcome.changed_files).to eq(["a.txt"])
    end

    it "exposes the verification" do
      expect(outcome.verification.status).to eq("failed")
    end

    it "exposes the response" do
      expect(outcome.response.value).to eq(1)
    end

    it "exposes the facts" do
      expect(scalars_of(outcome.facts)).to eq([["extra", true]])
    end

    context "with a verification that is not a Verification", contract: true do
      it "raises TypeError" do
        expect { build_outcome(verification: :unchecked) }.to raise_error(TypeError)
      end
    end

    context "with a native response and facts" do
      subject(:outcome) do
        build_outcome(response: {"answer" => 42}, facts: {state: {"balance" => 5}})
      end

      it "converts the response" do
        expect(outcome.response.get(path: ["answer"]).value).to eq(42)
      end

      it "converts the facts" do
        expect(outcome.facts[0][1].get(path: ["balance"]).value).to eq(5)
      end
    end

    context "with an unsupported fact value", contract: true do
      let(:facts) { [["state", Object.new]] }

      it "raises TypeError" do
        expect { outcome }.to raise_error(TypeError)
      end
    end

    context "with a fact that is not a pair", contract: true do
      let(:facts) { [["response"]] }

      it "raises TypeError" do
        expect { outcome }.to raise_error(TypeError)
      end
    end

    context "with facts as a Hash" do
      let(:facts) { {extra: kernel(true)} }

      it "names the facts" do
        expect(scalars_of(outcome.facts)).to eq([["extra", true]])
      end
    end

    context "with facts nested to the limit" do
      let(:deepest) { (1..255).reduce(kernel(nil)) { |inner, _| kernel([inner]) } }
      let(:observation) { build_outcome(response: deepest, facts: [["fact", deepest]]).observation }

      it "accepts a response at the limit" do
        expect(build_outcome(response: deepest).response).to be_a(GeniusBench::Value::List)
      end

      it "rejects a generated observation as a response" do
        expect { build_outcome(response: observation) }.to raise_error(ArgumentError, /nesting/)
      end

      it "rejects a generated observation as a fact" do
        expect { build_outcome(facts: [["fact", observation]]) }.to raise_error(ArgumentError, /nesting/)
      end
    end
  end

  describe "#observation" do
    subject(:observation) do
      build_outcome(
        changed_files: ["a.txt"],
        verification: GeniusBench::Verification::Failed.new,
        response: kernel(1),
        facts: [["verification", kernel("passed")], ["changed_files", kernel([])], ["extra", kernel(true)]]
      ).observation
    end

    it "places reserved channels before open facts" do
      expect(observation.fields.map(&:first)).to eq(%w[changed_files verification response verification changed_files extra])
    end

    it "keeps the verification status over a fact" do
      expect(observation.get(path: ["verification"]).value).to eq("failed")
    end

    it "keeps the changed files over a fact" do
      expect(observation.get(path: ["changed_files"]).items.map(&:value)).to eq(["a.txt"])
    end

    it "keeps open facts reachable" do
      expect(observation.get(path: ["extra"]).value).to be(true)
    end
  end
end
