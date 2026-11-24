fn update(input) {
    let phase = (input.time * 3.0) % 1.0;
    let bounce = if phase < 0.5 { phase * 2.0 + 0.4 } else { 2.4 - phase * 2.0 };
    #{ bounce: bounce }
}

