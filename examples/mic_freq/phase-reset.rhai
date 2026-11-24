fn update(input) {
    let beat = (input.time * 2.0) % 1.0;
    let bounce = if beat < 0.1 { 2.0 } else { 0.35 };
    #{ bounce: bounce }
}

