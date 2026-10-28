from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
CORE = ROOT / "crates/semantic-core/src"
SUM_TYPES = {
    "Value": "value.rs",
    "Predicate": "predicate.rs",
    "Verification": "outcome.rs",
    "Assessment": "assessment.rs",
    "Criterion": "task.rs",
}


def block(text, opening, indent):
    start = text.index(opening)
    end = text.find(f"\n{indent}end\n" if indent is not None else "\n}\n", start)
    return text[start:end]


def core_variants(name):
    body = block((CORE / SUM_TYPES[name]).read_text(), f"pub enum {name} {{", None)
    return {
        variant: re.findall(r"(\w+):", fields or "")
        for variant, fields in re.findall(
            r"^    (\w+)(?: \{([^}]*)\}|\(.*\))?,$", body, re.MULTILINE
        )
    }


def python_variants(name):
    stub = (ROOT / "bindings/python/python/kernel/__init__.pyi").read_text()
    start = stub.index(f"\nclass {name}:\n")
    end = re.compile(r"^\S", re.MULTILINE).search(stub, start + 2).start()
    return {
        variant: re.findall(r'"(\w+)"', fields)
        for variant, fields in re.findall(
            rf"    class (\w+)\({name}\):\n        __match_args__: Final = \(([^)]*)\)",
            stub[start:end],
        )
    }


def ruby_variants(name):
    signature = (ROOT / "bindings/ruby/sig/genius_bench.rbs").read_text()
    body = block(signature, f"  class {name}\n", "  ")
    return {
        variant: re.findall(r"(\w+): ", fields)
        for variant, fields in re.findall(
            rf"    class (\w+) < {name}\n      def self\.new: \(([^)]*)\)", body
        )
    }


class VariantParityTests(unittest.TestCase):
    """Python and Ruby expose one variant shape per core sum type."""

    def test_bindings_declare_every_core_variant_with_the_same_fields(self):
        for name in SUM_TYPES:
            with self.subTest(sum_type=name):
                core = core_variants(name)
                python = python_variants(name)
                ruby = ruby_variants(name)
                self.assertTrue(core)
                self.assertEqual(list(python), list(core))
                self.assertEqual(python, ruby)
                for variant, fields in core.items():
                    if fields:
                        self.assertEqual(python[variant], fields)


if __name__ == "__main__":
    unittest.main()
