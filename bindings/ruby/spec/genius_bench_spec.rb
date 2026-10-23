# frozen_string_literal: true

RSpec.describe GeniusBench do
  describe ".version" do
    subject { described_class.version }

    it { is_expected.to eq("0.1.0") }
    it { is_expected.to eq(GeniusBench::VERSION) }
  end

  describe ".source_hash" do
    subject { described_class.source_hash }

    it { is_expected.to match(/\A\h{64}\z/) }

    context "when the installed artifact declares its identity" do
      it "matches the declared hash" do
        skip "KERNEL_SOURCE_HASH is not set" unless ENV["KERNEL_SOURCE_HASH"]
        expect(subject).to eq(ENV.fetch("KERNEL_SOURCE_HASH"))
      end
    end
  end

  describe "REQUIRED_SCORES" do
    subject { described_class::REQUIRED_SCORES }

    it { is_expected.to eq(%w[correctness scope]) }
    it { is_expected.to be_frozen }
  end

  describe "the native library" do
    subject { $LOADED_FEATURES.grep(/semantic_ruby\.(bundle|so)\z/) }

    it { is_expected.to have_attributes(size: 1) }

    context "when the subprocess PATH holds no compiler" do
      it "finds no compiler commands" do
        skip "KERNEL_COMPILER_FREE is not set" unless ENV["KERNEL_COMPILER_FREE"]
        directories = ENV.fetch("PATH", "").split(::File::PATH_SEPARATOR)
        found = %w[cargo rustc ocamlc ocaml cc clang gcc].select do |command|
          directories.any? { |directory| ::File.executable?(::File.join(directory, command)) }
        end
        expect(found).to be_empty
      end
    end
  end

  describe ".from_struct" do
    let(:answer) { Struct.new(:answer) }
    let(:parent) { Data.define(:children) }

    context "with a Struct instance" do
      subject { described_class.from_struct(value: answer.new(42)) }

      it "projects members into a record" do
        expect(subject.get(path: ["answer"]).scalar).to eq(42)
      end
    end

    context "with nested Data and Struct instances" do
      subject { described_class.from_struct(value: parent.new(children: [answer.new(42)])) }

      it "projects every level" do
        expect(subject.get(path: ["children"]).items[0].get(path: ["answer"]).scalar).to eq(42)
      end
    end

    context "with a class that overrides to_h" do
      let(:overriding) do
        Class.new(Struct.new(:answer)) do
          def to_h = { "injected" => true }
        end
      end

      subject { described_class.from_struct(value: overriding.new(42)) }

      it "projects the declared members only" do
        expect(scalars_of(subject.fields)).to eq([["answer", 42]])
      end
    end

    context "with a Hash" do
      it "raises TypeError" do
        expect { described_class.from_struct(value: { "answer" => 42 }) }.to raise_error(TypeError)
      end
    end

    context "with a positional argument" do
      it "raises ArgumentError" do
        expect { described_class.from_struct(answer.new(42)) }.to raise_error(ArgumentError)
      end
    end

    context "with a cyclic struct" do
      let(:recursive) { answer.new([]).tap { |item| item.answer << item } }

      it "raises ArgumentError" do
        expect { described_class.from_struct(value: recursive) }.to raise_error(ArgumentError, /[Cc]ycl/)
      end
    end

    context "with nesting beyond the limit" do
      let(:deep) { (1..300).reduce(answer.new(nil)) { |inner, _| answer.new(inner) } }

      it "raises ArgumentError" do
        expect { described_class.from_struct(value: deep) }.to raise_error(ArgumentError, /nesting/)
      end
    end
  end

  describe "classes without constructors" do
    [GeniusBench::Value, GeniusBench::Case, GeniusBench::Issue, GeniusBench::Predicate].each do |klass|
      context "when calling #{klass}.new" do
        it "raises TypeError" do
          expect { klass.new }.to raise_error(TypeError, /allocator undefined/)
        end
      end
    end
  end
end
