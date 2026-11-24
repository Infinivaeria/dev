fn update(input) {
    let value = input.frame % 7;
    let bounce = if value == 0 { 0.4 }
        else if value == 1 { 0.8 }
        else if value == 2 { 1.2 }
        else if value == 3 { 1.6 }
        else if value == 4 { 2.0 }
        else if value == 5 { 1.2 }
        else { 0.6 };
    #{ bounce: bounce }
}

