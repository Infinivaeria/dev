fn update(input) {
    let phase = (input.time / 2.0).floor() % 3.0;
    let pose = if phase == 0.0 { "idle" } else if phase == 1.0 { "talking" } else { "loud" };
    #{ pose: pose }
}

