Raylib.init_window(480, 320, "Embedded bytecode game")
begin
  3.times do
    Raylib.begin_drawing
    Raylib.clear_background(Raylib::RAYWHITE)
    Raylib.draw_circle(240, 160, 40, Raylib::BLUE)
    Raylib.draw_text("Rust + Magnus + Ruby bytecode", 20, 24, 20, Raylib::DARKGRAY)
    Raylib.end_drawing
  end
  File.write("graphics-ok.txt", "rendered")
ensure
  Raylib.close_window
end