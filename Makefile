.PHONY: check install-hooks

CARGO ?= cargo
PYTHON ?= python3
UV ?= uv
KERNEL_PYTHON ?= 3.12

check:
	$(PYTHON) scripts/check.py --cargo "$(CARGO)" --uv "$(UV)" --python "$(KERNEL_PYTHON)"

install-hooks:
	git config core.hooksPath .githooks
