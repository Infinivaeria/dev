# Pong — plain raylib-bindings style. Run with:
#   selenite --game examples/games/pong.rb
# W/S and Up/Down move the paddles, Space serves, Esc quits.
require "raylib"
include Raylib

WIDTH = 960
HEIGHT = 540
PADDLE = [14, 90].freeze
SPEED = 420.0

SetConfigFlags(FLAG_MSAA_4X_HINT | FLAG_VSYNC_HINT)
InitWindow(WIDTH, HEIGHT, "Selenite Pong")
SetTargetFPS(60)

left = Rectangle.create(30, HEIGHT / 2 - 45, *PADDLE)
right = Rectangle.create(WIDTH - 44, HEIGHT / 2 - 45, *PADDLE)
ball = Vector2.create(WIDTH / 2, HEIGHT / 2)
velocity = Vector2.create(0, 0)
score = [0, 0]

until WindowShouldClose()
  dt = GetFrameTime()
  left.y += SPEED * dt if IsKeyDown(KEY_S)
  left.y -= SPEED * dt if IsKeyDown(KEY_W)
  right.y += SPEED * dt if IsKeyDown(KEY_DOWN)
  right.y -= SPEED * dt if IsKeyDown(KEY_UP)
  [left, right].each { |paddle| paddle.y = Clamp(paddle.y, 0.0, (HEIGHT - paddle.height).to_f) }
  if IsKeyPressed(KEY_SPACE) && velocity.x.zero?
    velocity = Vector2.create(360 * [-1, 1].sample, GetRandomValue(-200, 200))
  end

  ball += velocity * dt
  velocity.y = -velocity.y if ball.y < 8 || ball.y > HEIGHT - 8
  [left, right].each do |paddle|
    next unless CheckCollisionCircleRec(ball, 8, paddle)
    velocity.x = -velocity.x * 1.05
    velocity.y += (ball.y - paddle.center.y) * 4
    ball.x = paddle.equal?(left) ? paddle.x + paddle.width + 8 : paddle.x - 8
  end
  if ball.x < 0 || ball.x > WIDTH
    score[ball.x < 0 ? 1 : 0] += 1
    ball = Vector2.create(WIDTH / 2, HEIGHT / 2)
    velocity = Vector2.create(0, 0)
  end

  BeginDrawing()
  ClearBackground(DARKBLUE)
  DrawLine(WIDTH / 2, 0, WIDTH / 2, HEIGHT, Fade(WHITE, 0.3))
  DrawRectangleRec(left, RAYWHITE)
  DrawRectangleRec(right, RAYWHITE)
  DrawCircleV(ball, 8, GOLD)
  DrawText("#{score[0]}   #{score[1]}", WIDTH / 2 - 60, 20, 48, RAYWHITE)
  DrawText("Space to serve", WIDTH / 2 - 80, HEIGHT - 34, 20, LIGHTGRAY) if velocity.x.zero?
  DrawFPS(10, 10)
  EndDrawing()
end

CloseWindow()
