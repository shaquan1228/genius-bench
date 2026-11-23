.PHONY: all check check-go clean install-hooks

CARGO ?= cargo
PYTHON ?= python3
UV ?= uv
KERNEL_PYTHON ?= 3.12
BUNDLE ?= bundle
RUBY ?= ruby

all: check

check:
	$(PYTHON) scripts/check.py --cargo "$(CARGO)" --uv "$(UV)" --python "$(KERNEL_PYTHON)" --bundle "$(BUNDLE)" --ruby "$(RUBY)"

check-go:
	$(PYTHON) bindings/go/check.py --cargo "$(CARGO)"

clean:
	rm -rf target bindings/go/native/target bindings/ruby/target bindings/ruby/tmp bindings/ruby/pkg bindings/ruby/LICENSE bindings/python/dist

install-hooks:
	git config core.hooksPath .githooks
