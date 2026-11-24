fn update(input) {
    if input.adjusted_db > input.gate_db + 3.0 && input.pose == "idle" {
        #{ pose: "talking" }
    } else {
        #{ pose: "auto" }
    }
}

