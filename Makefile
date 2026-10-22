.PHONY: check install-hooks capture-fixtures

CARGO ?= cargo
PYTHON ?= python3
OCAMLC ?= ocamlc
UV ?= uv
KERNEL_PYTHON ?= 3.12

check:
	$(PYTHON) scripts/check.py --cargo "$(CARGO)" --uv "$(UV)" --python "$(KERNEL_PYTHON)"

install-hooks:
	git config core.hooksPath .githooks

capture-fixtures:
	$(PYTHON) scripts/capture_ocaml.py --source-repo "$(SOURCE_REPO)" --ocamlc "$(OCAMLC)"
