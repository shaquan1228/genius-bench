# frozen_string_literal: true

RSpec.describe GeniusBench::Record do
  describe ".new" do
    context "with duplicate names" do
      subject(:record) { described_class.new(fields: [["x", 1], ["x", 2]]) }

      it "keeps every field" do
        expect(record.fields.size).to eq(2)
      end

      it "returns frozen fields" do
        expect(record.fields).to be_frozen
      end
    end

    context "with a nested array that changes later" do
      let(:fields) { [["x", [1]]] }

      it "snapshots the array at construction" do
        record = described_class.new(fields: fields)
        fields[0][1] << 2
        expect(record.fields[0][1].items.size).to eq(1)
      end
    end

    context "with a symbol name" do
      subject(:record) { described_class.new(fields: [[:name, 1]]) }

      it "converts the name" do
        expect(scalars_of(record.fields)).to eq([["name", 1]])
      end
    end

    context "with a Hash of fields" do
      subject(:record) { described_class.new(fields: { name: 1, "text" => "x" }) }

      it "keeps the entry order" do
        expect(scalars_of(record.fields)).to eq([["name", 1], ["text", "x"]])
      end
    end

    context "with a name that is not a String or Symbol" do
      it "raises TypeError" do
        expect { described_class.new(fields: [[1, 1]]) }.to raise_error(TypeError)
      end
    end

    context "with fields that are not pairs or a Hash" do
      it "raises TypeError" do
        expect { described_class.new(fields: "x") }.to raise_error(TypeError)
      end
    end

    context "with an entry that is not a pair" do
      it "raises TypeError" do
        expect { described_class.new(fields: ["x"]) }.to raise_error(TypeError)
      end
    end

    context "with native fields composed to the limit" do
      let(:deepest) { (1..256).reduce(kernel(nil)) { |inner, _| described_class.new(fields: [["x", inner]]) } }

      it "accepts the limit" do
        expect(kernel(deepest).kind).to eq("record")
      end

      it "rejects one more level" do
        expect { described_class.new(fields: [["x", deepest]]) }.to raise_error(ArgumentError, /nesting/)
      end
    end

    context "when a callback replaces a later pair" do
      let(:tail) { "tail " * 1024 }
      let(:source) { [["hook", hook], ["tail", tail.dup]] }
      let(:hook) do
        source_reference = -> { source }
        Object.new.tap do |object|
          object.define_singleton_method(:to_kernel_value) do
            pairs = source_reference.call
            pairs << ["added", 3]
            pairs[1] = ["replaced", 4]
            GC.start
            GC.compact if GC.respond_to?(:compact)
            1
          end
        end
      end

      it "converts the snapshot" do
        expect(scalars_of(described_class.new(fields: source).fields)).to eq([["hook", 1], ["tail", tail]])
      end
    end

    context "with pairs inside an array that overrides iteration" do
      let(:list) do
        Class.new(Array) do
          def each = raise("Overridden array iterator")
        end
      end

      it "reads pairs natively" do
        expect(described_class.new(fields: list.new([list.new(["x", 1])])).fields[0][1].scalar).to eq(1)
      end
    end
  end

  describe "#to_kernel_value" do
    subject { described_class.new(fields: [["x", 1]]).to_kernel_value }

    it { is_expected.to be_a(GeniusBench::Value) }

    it "keeps the fields" do
      expect(scalars_of(subject.fields)).to eq([["x", 1]])
    end
  end
end
