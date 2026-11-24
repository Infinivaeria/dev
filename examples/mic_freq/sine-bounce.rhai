fn update(input) {
    let wave = (input.time * 6.283185).sin();
    #{ bounce: 1.0 + wave * 0.65 }
}

