// Runs at the render rate. Available input fields:
// frame, time, dt, db, adjusted_db, pitch_hz, band, pose, muted,
// gate_db, loud_delta_db.
fn update(input) {
    let output = #{};

    // Keep a bright voice in the high artwork set.
    if input.pitch_hz != () && input.pitch_hz > 360.0 {
        output.band = "high";
        output.bounce = 1.25;
    }

    // Disable strong bounce on nearly silent input.
    if input.adjusted_db < input.gate_db {
        output.bounce = 0.15;
    }

    output
}
