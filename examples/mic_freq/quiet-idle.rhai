fn update(input) {
    if input.adjusted_db < input.gate_db + 2.0 {
        #{ pose: "idle", bounce: 0.15 }
    } else {
        #{ pose: "auto" }
    }
}

