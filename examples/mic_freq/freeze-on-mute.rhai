fn update(input) {
    if input.muted {
        #{ pose: "idle", bounce: 0.0, scale: 0.9, blink: false }
    } else {
        #{ pose: "auto" }
    }
}

