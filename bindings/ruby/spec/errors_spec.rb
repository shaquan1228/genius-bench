# frozen_string_literal: true

RSpec.describe GeniusBench::ValidationError do
  it { is_expected.to be_a(ArgumentError) }
end

RSpec.describe GeniusBench::GradingError do
  it { is_expected.to be_a(RuntimeError) }
end
