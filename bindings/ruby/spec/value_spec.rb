# frozen_string_literal: true

RSpec.describe GeniusBench::Value do
  describe ".from_ruby" do
    context "with scalars" do
      {
        nil => "null", true => "bool", false => "bool", 1 => "int", 1.5 => "float", "x" => "string"
      }.each do |ruby, kind|
        it "converts #{ruby.inspect} to #{kind}" do
          expect(kernel(ruby).kind).to eq(kind)
        end
      end

      it "returns the integer unchanged" do
        expect(kernel(7).scalar).to eq(7)
      end

      it "returns nil for null" do
        expect(kernel(nil).scalar).to be_nil
      end
    end

    context "with integers at the i64 bounds" do
      [-(2**63), -(2**53) - 1, 2**53 - 1, 2**53, 2**53 + 1, 2**63 - 1].each do |number|
        it "keeps #{number} exact" do
          expect(kernel(number).scalar).to eql(number)
        end
      end
    end

    context "with integers outside i64" do
      [-(2**63) - 1, 2**63].each do |number|
        it "raises RangeError for #{number}" do
          expect { kernel(number) }.to raise_error(RangeError)
        end
      end
    end

    context "with floats" do
      [0x3FB999999999999A, 0x3FB999999999999B, 0x4340000000000000, 0x4340000000000001].each do |bits|
        it "keeps bit pattern #{bits.to_s(16)}" do
          number = [bits].pack("Q>").unpack1("G")
          expect([kernel(number).scalar].pack("G")).to eq([bits].pack("Q>"))
        end
      end
    end

    context "with strings" do
      it "keeps UTF-8 text" do
        expect(kernel("héllo 日本語").scalar).to eq("héllo 日本語")
      end

      it "transcodes compatible encodings" do
        expect(kernel("héllo".encode("ISO-8859-1")).scalar).to eq("héllo")
      end

      it "rejects binary strings with high bytes" do
        expect { kernel("héllo".b) }.to raise_error(EncodingError)
      end

      it "rejects invalid UTF-8" do
        expect { kernel((+"\xff").force_encoding("UTF-8")) }.to raise_error(EncodingError)
      end

      it "returns frozen scalars" do
        expect(kernel("x").scalar).to be_frozen
      end
    end

    context "with containers" do
      subject(:converted) { kernel({"values" => nested, :null => nil}) }

      let(:nested) { [1] }

      it "snapshots nested arrays before later mutation" do
        converted
        nested << 2
        expect(converted.get(path: ["values"]).items.size).to eq(1)
      end

      it "returns nil for a missing path" do
        expect(converted.get(path: ["missing"])).to be_nil
      end

      it "keeps an explicit null distinct from missing" do
        expect(converted.get(path: ["null"]).kind).to eq("null")
      end

      it "accepts symbol keys as names" do
        expect(scalars_of(kernel({x: 1}).fields)).to eq([["x", 1]])
      end

      it "rejects names that collide after symbol conversion" do
        expect { kernel({"x" => 1, :x => 2}) }.to raise_error(ArgumentError, /Duplicate record field/)
      end

      it "keeps shared sub-structures as separate items" do
        expect(kernel([nested, nested]).items.size).to eq(2)
      end
    end

    context "with a to_kernel_value object" do
      let(:custom) { Object.new.tap { |object| def object.to_kernel_value = {"answer" => 42} } }

      it "converts the protocol result" do
        expect(kernel(custom).get(path: ["answer"]).scalar).to eq(42)
      end
    end

    context "with cycles" do
      let(:recursive) { [].tap { |array| array << array } }
      let(:self_returning) { Object.new.tap { |object| def object.to_kernel_value = self } }

      it "rejects a self-containing array", contract: true do
        expect { kernel(recursive) }.to raise_error(ArgumentError, /[Cc]ycl/)
      end

      it "rejects a protocol object returning itself" do
        expect { kernel(self_returning) }.to raise_error(ArgumentError, /[Cc]ycl/)
      end
    end

    context "with unsupported values", contract: true do
      [Object.new, {1 => "bad"}, Set.new([1, 2]), :symbol].each do |unsupported|
        it "raises TypeError for #{unsupported.class}" do
          expect { kernel(unsupported) }.to raise_error(TypeError)
        end
      end
    end

    context "with the keyword contract", contract: true do
      it "rejects a positional value" do
        expect { described_class.from_ruby(1) }.to raise_error(ArgumentError)
      end

      it "rejects extra keywords" do
        expect { described_class.from_ruby(value: 1, extra: 2) }.to raise_error(ArgumentError)
      end
    end

    context "when a callback mutates a hash during conversion" do
      let(:tail) { "tail " * 1024 }
      let(:source) { {"hook" => hook, "tail" => tail.dup} }
      let(:hook) do
        source_reference = -> { source }
        action = mutation
        Object.new.tap do |object|
          object.define_singleton_method(:to_kernel_value) do
            action.call(source_reference.call)
            GC.start
            GC.compact if GC.respond_to?(:compact)
            1
          end
        end
      end

      context "when it adds a key" do
        let(:mutation) { ->(hash) { hash["added"] = 3 } }

        it "converts the snapshot" do
          expect(scalars_of(kernel(source).fields)).to eq([["hook", 1], ["tail", tail]])
        end
      end

      context "when it removes a heap value" do
        let(:mutation) { ->(hash) { hash.delete("tail") } }

        it "keeps the removed value alive" do
          expect(scalars_of(kernel(source).fields)).to eq([["hook", 1], ["tail", tail]])
        end
      end
    end

    context "when a callback mutates an array during conversion" do
      let(:tail) { "tail " * 1024 }
      let(:source) { [hook, tail.dup] }
      let(:hook) do
        source_reference = -> { source }
        Object.new.tap do |object|
          object.define_singleton_method(:to_kernel_value) do
            source_reference.call << 3
            GC.start
            1
          end
        end
      end

      it "converts the snapshot" do
        expect(kernel(source).items.map(&:scalar)).to eq([1, tail])
      end
    end

    context "when a callback clears the array being converted" do
      let(:items) { Array.new(64) { |index| "item #{index}" } }
      let(:source) { [hook, *items] }
      let(:hook) do
        source_reference = -> { source }
        Object.new.tap do |object|
          object.define_singleton_method(:to_kernel_value) do
            source_reference.call.clear
            GC.start
            "hooked"
          end
        end
      end

      it "converts every snapshot element" do
        expect(kernel(source).items.map(&:scalar)).to eq(["hooked", *items])
      end
    end

    context "with containers that override iteration", contract: true do
      let(:list) do
        Class.new(Array) do
          def each = raise("Overridden array iterator")
          def [](*) = raise("Overridden array reader")
        end
      end
      let(:mapping) do
        Class.new(Hash) do
          def each = raise("Overridden hash iterator")
          def each_pair = raise("Overridden hash iterator")
        end
      end

      it "reads arrays natively" do
        expect(kernel(list.new([1, 2])).items.map(&:scalar)).to eq([1, 2])
      end

      it "reads hashes natively" do
        expect(scalars_of(kernel(mapping[{"x" => 1}]).fields)).to eq([["x", 1]])
      end
    end

    context "with nesting beyond the limit" do
      let(:deep) { (1..300).reduce(nil) { |inner, _| [inner] } }

      it "raises ArgumentError" do
        expect { kernel(deep) }.to raise_error(ArgumentError, /nesting/)
      end
    end

    context "with native values composed to the limit" do
      let(:deepest) { (1..256).reduce(kernel(nil)) { |inner, _| kernel([inner]) } }

      it "accepts the limit" do
        expect(kernel(deepest).kind).to eq("list")
      end

      it "rejects one more level" do
        expect { kernel([deepest]) }.to raise_error(ArgumentError, /nesting/)
      end
    end
  end

  describe "#scalar" do
    it "raises TypeError for a structured value" do
      expect { kernel([]).scalar }.to raise_error(TypeError)
    end
  end

  describe "#items" do
    it "raises TypeError for a non-list" do
      expect { kernel(1).items }.to raise_error(TypeError)
    end

    it "returns a frozen snapshot" do
      expect(kernel([1]).items).to be_frozen
    end
  end

  describe "#fields" do
    it "raises TypeError for a non-record" do
      expect { kernel(1).fields }.to raise_error(TypeError)
    end
  end

  describe "#equal_to?" do
    let(:duplicate) { kernel(GeniusBench::Record.new(fields: [["x", 1], ["x", 2]])) }

    it "uses first-field record semantics" do
      expect(duplicate).to be_equal_to(other: kernel({"x" => 1}))
    end

    it "keeps booleans distinct from integers" do
      expect(kernel(true)).not_to be_equal_to(other: kernel(1))
    end

    it "keeps integers distinct from floats" do
      expect(kernel(1)).not_to be_equal_to(other: kernel(1.0))
    end

    it "treats NaN as unequal to itself" do
      nan = kernel(Float::NAN)
      expect(nan).not_to be_equal_to(other: nan)
    end

    it "treats signed zeros as equal" do
      expect(kernel(-0.0)).to be_equal_to(other: kernel(0.0))
    end

    it "treats infinities as equal" do
      expect(kernel(Float::INFINITY)).to be_equal_to(other: kernel(Float::INFINITY))
    end

    it "leaves identity equality to Ruby" do
      expect(duplicate).not_to equal(kernel(GeniusBench::Record.new(fields: [["x", 1], ["x", 2]])))
    end

    it "compares a native value" do
      expect(kernel({"x" => 1})).to be_equal_to(other: {"x" => 1})
    end

    it "rejects an unsupported value", contract: true do
      expect { duplicate.equal_to?(other: Object.new) }.to raise_error(TypeError)
    end
  end

  describe "#==" do
    subject(:value) { kernel({"x" => 1, "y" => [1.5, nil]}) }

    it { is_expected.to eq(kernel({"y" => [1.5, nil], "x" => 1})) }
    it { is_expected.not_to eq(kernel({"x" => 1})) }
    it { is_expected.not_to eq({"x" => 1, "y" => [1.5, nil]}) }
    it { is_expected.to eql(kernel({"x" => 1, "y" => [1.5, nil]})) }

    it "keeps NaN unequal to itself" do
      nan = kernel(Float::NAN)
      expect(nan).not_to eq(nan)
    end
  end

  describe "#hash" do
    it "agrees with == across field order" do
      expect(kernel({"x" => 1, "y" => 2}).hash).to eq(kernel({"y" => 2, "x" => 1}).hash)
    end

    it "agrees with == across signed zeros" do
      expect(kernel(-0.0).hash).to eq(kernel(0.0).hash)
    end

    it "agrees with == across duplicate fields" do
      duplicate = kernel(GeniusBench::Record.new(fields: [["x", 1], ["x", 2]]))
      expect(duplicate.hash).to eq(kernel({"x" => 1}).hash)
    end

    it "deduplicates equal values in a Hash" do
      expect({kernel([1]) => true, kernel([1]) => false}.size).to eq(1)
    end
  end

  describe "#inspect" do
    subject { kernel([1, "x"]).inspect }

    it { is_expected.to eq('#<GeniusBench::Value List([Int(1), String("x")])>') }
  end
end
