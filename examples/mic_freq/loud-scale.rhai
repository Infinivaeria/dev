fn update(input) {
    let scale = 0.72 + (input.adjusted_db - input.gate_db) * 0.015;
    if scale < 0.72 { scale = 0.72; }
    if scale > 1.0 { scale = 1.0; }
    #{ scale: scale }
}

