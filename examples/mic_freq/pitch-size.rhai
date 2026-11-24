fn update(input) {
    let scale = if input.band == "low" { 0.82 } else if input.band == "high" { 1.0 } else { 0.9 };
    #{ scale: scale }
}

