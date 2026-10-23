# frozen_string_literal: true

RSpec.describe GeniusBench::Task do
  describe ".new" do
    subject(:task) do
      described_class.new(
        id: "code",
        fixture: fixture,
        instruction: GeniusBench::Instruction.new(prompt: "change"),
        criterion: GeniusBench::Criterion.code(verifier: verifier),
        allowed_files: paths,
        reference: GeniusBench::Reference.new(edits: [], completion: kernel("secret"))
      )
    end

    let(:paths) { ["a.txt"] }
    let(:fixture) { [GeniusBench::File.new(path: "a.txt", contents: "original")] }
    let(:tools) { [GeniusBench::File.new(path: "tool", contents: "public")] }
    let(:verifier) do
      GeniusBench::Verifier.new(
        inputs: paths,
        files: [GeniusBench::File.new(path: "private", contents: "secret")],
        tools: tools,
        command: ["check"],
        stdin: "secret",
        timeout_seconds: 1
      )
    end

    it { is_expected.to be_frozen }
    it { is_expected.to have_attributes(id: "code", allowed_files: ["a.txt"]) }

    context "when the inputs change after construction" do
      before do
        task
        paths << "other"
        fixture.clear
        tools.clear
      end

      it "keeps the allowed files" do
        expect(task.allowed_files).to eq(["a.txt"])
      end

      it "keeps the fixture" do
        expect(task.fixture.map(&:path)).to eq(["a.txt"])
      end

      it "keeps the public tools" do
        expect(task.project.files.map(&:path)).to eq(%w[a.txt tool])
      end
    end

    it "returns frozen collections" do
      expect(task.allowed_files).to be_frozen
    end

    it "returns frozen strings" do
      expect(task.id).to be_frozen
    end

    it "rejects attribute writers" do
      expect { task.id = "changed" }.to raise_error(NoMethodError)
    end

    it "rejects new instance variables" do
      expect { task.instance_variable_set(:@extra, 1) }.to raise_error(FrozenError)
    end

    describe "#project" do
      subject(:projected) { task.project }

      it { is_expected.to have_attributes(case_id: "code", prompt: "change") }

      it "omits the reference" do
        expect(projected).not_to respond_to(:reference)
      end

      it "omits the private checks" do
        expect(projected).not_to respond_to(:checks)
      end

      it "omits any consumer label" do
        expect(projected).not_to respond_to(:skill)
      end
    end

    describe "#checks" do
      it "exposes the verifier for code criteria" do
        expect(task.checks.stdin).to eq("secret")
      end

      it "is nil for state criteria" do
        expect(state_task.checks).to be_nil
      end
    end

    context "with an invalid declaration" do
      let(:invalid) do
        lambda do
          described_class.new(
            id: "bad id",
            fixture: [GeniusBench::File.new(path: "../bad", contents: "")],
            instruction: GeniusBench::Instruction.new(prompt: ""),
            criterion: GeniusBench::Criterion.code(
              verifier: GeniusBench::Verifier.new(inputs: [], files: [], tools: [], command: [], stdin: "", timeout_seconds: 0)
            ),
            allowed_files: [],
            reference: GeniusBench::Reference.new(edits: [], completion: kernel(nil))
          )
        end
      end
      let(:error) { invalid.call rescue $! }

      it "raises ValidationError" do
        expect { invalid.call }.to raise_error(GeniusBench::ValidationError)
      end

      it "retains the case identity" do
        expect(error.case_id).to eq("bad id")
      end

      it "retains every applicable issue" do
        expect(error.issues.map(&:field)).to eq(%w[id fixture verifier.inputs verifier.command verifier.timeout_seconds])
      end

      it "returns frozen issues" do
        expect(error.issues).to be_frozen
      end

      it "formats the message from the issues" do
        expect(error.message).to match(/case "bad id": id: Invalid case identity; fixture:/)
      end

      it "keeps issues immutable" do
        expect { error.issues[0].field = "changed" }.to raise_error(NoMethodError)
      end
    end

    context "with a predicate nested beyond the limit" do
      let(:deep) { (1..256).reduce(GeniusBench::Predicate.present(path: [])) { |inner, _| GeniusBench::Predicate.not(check: inner) } }

      it "raises ArgumentError" do
        expect { state_task(predicate: deep) }.to raise_error(ArgumentError, /nesting/)
      end
    end

    context "with a fixture entry that is not a File" do
      let(:fixture) { ["no"] }

      it "raises TypeError" do
        expect { task }.to raise_error(TypeError)
      end
    end
  end

  describe "#grade" do
    subject(:task) { state_task }

    let(:graded) { build_outcome(response: kernel({ "answer" => 42 }), facts: [["response", kernel(nil)]]) }

    it "scores correctness and scope" do
      expect(task.grade(outcome: graded)).to eq([["correctness", true], ["scope", true]])
    end

    it "returns frozen scores" do
      expect(task.grade(outcome: graded)).to be_frozen
    end

    it "returns frozen score pairs" do
      expect(task.grade(outcome: graded)[0]).to be_frozen
    end

    it "requires the outcome keyword" do
      expect { task.grade(graded) }.to raise_error(ArgumentError)
    end

    %w[failed timed_out invalid_submission].each do |status|
      context "when the verification #{status}" do
        let(:outcome) { build_outcome(verification: GeniusBench::Verification.public_send(status)) }

        it "scores correctness false" do
          expect(task.grade(outcome: outcome)).to eq([["correctness", false], ["scope", true]])
        end
      end
    end

    context "when the harness failed" do
      let(:outcome) { build_outcome(verification: GeniusBench::Verification.harness_error(detail: "worker missing")) }
      let(:error) { task.grade(outcome: outcome) rescue $! }

      it "raises GradingError" do
        expect { task.grade(outcome: outcome) }.to raise_error(GeniusBench::GradingError)
      end

      it { expect(error.detail).to eq("worker missing") }
      it { expect(error.failure).to eq("verification_harness_error") }
      it { expect(error.message).to eq("Verification harness error: worker missing") }
    end

    context "under garbage collector stress" do
      subject(:task) { state_task(predicate: GeniusBench::Predicate.all(checks: checks)) }

      let(:checks) { Array.new(20) { |index| [index.to_s, GeniusBench::Predicate.present(path: ["response"])] } }
      let(:entries) { Array.new(20) { |index| { "index" => index, "text" => "entry #{index}" } } }

      around do |example|
        GC.stress = true
        example.run
      ensure
        GC.stress = false
      end

      it "grades correctly" do
        expect(task.grade(outcome: build_outcome(response: kernel({ "answer" => 42 })))).to eq([["correctness", true], ["scope", true]])
      end

      it "converts nested records intact" do
        converted = kernel(entries).items.map { |item| scalars_of(item.fields) }
        expect(converted.last).to eq([["index", 19], ["text", "entry 19"]])
      end

      it "exposes assessment checks intact" do
        assessment = task.assess(outcome: build_outcome(response: kernel({ "answer" => 42 })))
        expect(assessment.checks[0][1].checks.map(&:first)).to eq(Array.new(20, &:to_s))
      end
    end
  end

  describe "#assess" do
    subject(:task) { state_task }

    it "returns a passing assessment for a matching response" do
      expect(task.assess(outcome: build_outcome(response: kernel({ "answer" => 42 })))).to be_passed
    end

    it "requires the outcome keyword" do
      expect { task.assess(build_outcome) }.to raise_error(ArgumentError)
    end
  end
end
