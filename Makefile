.PHONY: check install-hooks capture-fixtures

CARGO ?= cargo
PYTHON ?= python3
OCAMLC ?= ocamlc

check:
	$(PYTHON) scripts/check.py --cargo "$(CARGO)"

install-hooks:
	git config core.hooksPath .githooks

capture-fixtures:
	$(PYTHON) scripts/capture_ocaml.py --source-repo "$(SOURCE_REPO)" --ocamlc "$(OCAMLC)"
