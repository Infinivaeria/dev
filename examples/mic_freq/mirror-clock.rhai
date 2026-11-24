fn update(input) {
    let mirrored = (input.time / 4.0).floor() % 2.0 == 1.0;
    #{ mirror: mirrored }
}

