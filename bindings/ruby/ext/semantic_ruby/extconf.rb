# frozen_string_literal: true

require "mkmf"
require "rb_sys/mkmf"

create_rust_makefile("genius_bench/semantic_ruby") do |extension|
  extension.extra_cargo_args += ["--locked"]
end
