fn update(input) {
    #{ blink: input.pose != "idle" && !input.muted }
}

