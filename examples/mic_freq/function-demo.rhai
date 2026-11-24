fn clamp_value(value) {
    if value < 0.2 { return 0.2; }
    if value > 2.0 { return 2.0; }
    value
}

fn update(input) {
    #{ bounce: clamp_value((input.db + 60.0) * 0.04) }
}
