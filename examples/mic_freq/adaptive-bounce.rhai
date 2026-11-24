fn update(input) {
    let level = input.adjusted_db;
    let bounce = 0.25;
    if level > input.gate_db + 6.0 { bounce = 0.8; }
    if level > input.gate_db + 12.0 { bounce = 1.3; }
    if level > input.gate_db + 24.0 { bounce = 1.8; }
    #{ bounce: bounce }
}

