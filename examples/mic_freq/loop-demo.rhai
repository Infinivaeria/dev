fn update(input) {
    let total = 0;
    let n = 1;
    while n <= 20 {
        total += n;
        n += 1;
    }
    #{ scale: 0.7 + (total % 10) * 0.03 }
}

