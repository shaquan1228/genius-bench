.PHONY: check install-hooks

CARGO ?= cargo
PYTHON ?= python3

check:
	$(PYTHON) scripts/check.py --cargo "$(CARGO)"

install-hooks:
	git config core.hooksPath .githooks
