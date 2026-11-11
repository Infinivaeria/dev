# Complete Raylib API

Generated from the linked raylib public header. All 600 functions are available under `Selenite.raylib_native` after `enable!`. Native calls use C ownership rules; do not mix native resources with managed handles.

See the native API tutorial for records, pointers, callbacks and resource lifetime.

## InitWindow

```c
void InitWindow(int width, int height, const char * title);
```

Window-related functions

## CloseWindow

```c
void CloseWindow();
```



## WindowShouldClose

```c
_Bool WindowShouldClose();
```



## IsWindowReady

```c
_Bool IsWindowReady();
```



## IsWindowFullscreen

```c
_Bool IsWindowFullscreen();
```



## IsWindowHidden

```c
_Bool IsWindowHidden();
```



## IsWindowMinimized

```c
_Bool IsWindowMinimized();
```



## IsWindowMaximized

```c
_Bool IsWindowMaximized();
```



## IsWindowFocused

```c
_Bool IsWindowFocused();
```



## IsWindowResized

```c
_Bool IsWindowResized();
```



## IsWindowState

```c
_Bool IsWindowState(unsigned int flag);
```



## SetWindowState

```c
void SetWindowState(unsigned int flags);
```



## ClearWindowState

```c
void ClearWindowState(unsigned int flags);
```



## ToggleFullscreen

```c
void ToggleFullscreen();
```



## ToggleBorderlessWindowed

```c
void ToggleBorderlessWindowed();
```



## MaximizeWindow

```c
void MaximizeWindow();
```



## MinimizeWindow

```c
void MinimizeWindow();
```



## RestoreWindow

```c
void RestoreWindow();
```



## SetWindowIcon

```c
void SetWindowIcon(Image image);
```



## SetWindowIcons

```c
void SetWindowIcons(Image * images, int count);
```



## SetWindowTitle

```c
void SetWindowTitle(const char * title);
```



## SetWindowPosition

```c
void SetWindowPosition(int x, int y);
```



## SetWindowMonitor

```c
void SetWindowMonitor(int monitor);
```



## SetWindowMinSize

```c
void SetWindowMinSize(int width, int height);
```



## SetWindowMaxSize

```c
void SetWindowMaxSize(int width, int height);
```



## SetWindowSize

```c
void SetWindowSize(int width, int height);
```



## SetWindowOpacity

```c
void SetWindowOpacity(float opacity);
```



## SetWindowFocused

```c
void SetWindowFocused();
```



## GetWindowHandle

```c
void * GetWindowHandle();
```



## GetScreenWidth

```c
int GetScreenWidth();
```



## GetScreenHeight

```c
int GetScreenHeight();
```



## GetRenderWidth

```c
int GetRenderWidth();
```



## GetRenderHeight

```c
int GetRenderHeight();
```



## GetMonitorCount

```c
int GetMonitorCount();
```



## GetCurrentMonitor

```c
int GetCurrentMonitor();
```



## GetMonitorPosition

```c
Vector2 GetMonitorPosition(int monitor);
```



## GetMonitorWidth

```c
int GetMonitorWidth(int monitor);
```



## GetMonitorHeight

```c
int GetMonitorHeight(int monitor);
```



## GetMonitorPhysicalWidth

```c
int GetMonitorPhysicalWidth(int monitor);
```



## GetMonitorPhysicalHeight

```c
int GetMonitorPhysicalHeight(int monitor);
```



## GetMonitorRefreshRate

```c
int GetMonitorRefreshRate(int monitor);
```



## GetWindowPosition

```c
Vector2 GetWindowPosition();
```



## GetWindowScaleDPI

```c
Vector2 GetWindowScaleDPI();
```



## GetMonitorName

```c
const char * GetMonitorName(int monitor);
```



## SetClipboardText

```c
void SetClipboardText(const char * text);
```



## GetClipboardText

```c
const char * GetClipboardText();
```



## GetClipboardImage

```c
Image GetClipboardImage();
```



## EnableEventWaiting

```c
void EnableEventWaiting();
```



## DisableEventWaiting

```c
void DisableEventWaiting();
```



## ShowCursor

```c
void ShowCursor();
```

Cursor-related functions

## HideCursor

```c
void HideCursor();
```



## IsCursorHidden

```c
_Bool IsCursorHidden();
```



## EnableCursor

```c
void EnableCursor();
```



## DisableCursor

```c
void DisableCursor();
```



## IsCursorOnScreen

```c
_Bool IsCursorOnScreen();
```



## ClearBackground

```c
void ClearBackground(Color color);
```

Drawing-related functions

## BeginDrawing

```c
void BeginDrawing();
```



## EndDrawing

```c
void EndDrawing();
```



## BeginMode2D

```c
void BeginMode2D(Camera2D camera);
```



## EndMode2D

```c
void EndMode2D();
```



## BeginMode3D

```c
void BeginMode3D(Camera3D camera);
```



## EndMode3D

```c
void EndMode3D();
```



## BeginTextureMode

```c
void BeginTextureMode(RenderTexture2D target);
```



## EndTextureMode

```c
void EndTextureMode();
```



## BeginShaderMode

```c
void BeginShaderMode(Shader shader);
```



## EndShaderMode

```c
void EndShaderMode();
```



## BeginBlendMode

```c
void BeginBlendMode(int mode);
```



## EndBlendMode

```c
void EndBlendMode();
```



## BeginScissorMode

```c
void BeginScissorMode(int x, int y, int width, int height);
```



## EndScissorMode

```c
void EndScissorMode();
```



## BeginVrStereoMode

```c
void BeginVrStereoMode(VrStereoConfig config);
```



## EndVrStereoMode

```c
void EndVrStereoMode();
```



## LoadVrStereoConfig

```c
VrStereoConfig LoadVrStereoConfig(VrDeviceInfo device);
```

VR stereo config functions for VR simulator

## UnloadVrStereoConfig

```c
void UnloadVrStereoConfig(VrStereoConfig config);
```



## LoadShader

```c
Shader LoadShader(const char * vsFileName, const char * fsFileName);
```

Shader management functions NOTE: Shader functionality is not available on OpenGL 1.1

## LoadShaderFromMemory

```c
Shader LoadShaderFromMemory(const char * vsCode, const char * fsCode);
```



## IsShaderValid

```c
_Bool IsShaderValid(Shader shader);
```



## GetShaderLocation

```c
int GetShaderLocation(Shader shader, const char * uniformName);
```



## GetShaderLocationAttrib

```c
int GetShaderLocationAttrib(Shader shader, const char * attribName);
```



## SetShaderValue

```c
void SetShaderValue(Shader shader, int locIndex, const void * value, int uniformType);
```



## SetShaderValueV

```c
void SetShaderValueV(Shader shader, int locIndex, const void * value, int uniformType, int count);
```



## SetShaderValueMatrix

```c
void SetShaderValueMatrix(Shader shader, int locIndex, Matrix mat);
```



## SetShaderValueTexture

```c
void SetShaderValueTexture(Shader shader, int locIndex, Texture2D texture);
```



## UnloadShader

```c
void UnloadShader(Shader shader);
```



## GetScreenToWorldRay

```c
Ray GetScreenToWorldRay(Vector2 position, Camera camera);
```



## GetScreenToWorldRayEx

```c
Ray GetScreenToWorldRayEx(Vector2 position, Camera camera, int width, int height);
```



## GetWorldToScreen

```c
Vector2 GetWorldToScreen(Vector3 position, Camera camera);
```



## GetWorldToScreenEx

```c
Vector2 GetWorldToScreenEx(Vector3 position, Camera camera, int width, int height);
```



## GetWorldToScreen2D

```c
Vector2 GetWorldToScreen2D(Vector2 position, Camera2D camera);
```



## GetScreenToWorld2D

```c
Vector2 GetScreenToWorld2D(Vector2 position, Camera2D camera);
```



## GetCameraMatrix

```c
Matrix GetCameraMatrix(Camera camera);
```



## GetCameraMatrix2D

```c
Matrix GetCameraMatrix2D(Camera2D camera);
```



## SetTargetFPS

```c
void SetTargetFPS(int fps);
```

Timing-related functions

## GetFrameTime

```c
float GetFrameTime();
```



## GetTime

```c
double GetTime();
```



## GetFPS

```c
int GetFPS();
```



## SwapScreenBuffer

```c
void SwapScreenBuffer();
```

Custom frame control functions NOTE: Those functions are intended for advanced users that want full control over the frame processing By default EndDrawing() does this job: draws everything + SwapScreenBuffer() + manage frame timing + PollInputEvents() To avoid that behaviour and control frame processes manually, enable in config.h: SUPPORT_CUSTOM_FRAME_CONTROL

## PollInputEvents

```c
void PollInputEvents();
```



## WaitTime

```c
void WaitTime(double seconds);
```



## SetRandomSeed

```c
void SetRandomSeed(unsigned int seed);
```

Random values generation functions

## GetRandomValue

```c
int GetRandomValue(int min, int max);
```



## LoadRandomSequence

```c
int * LoadRandomSequence(unsigned int count, int min, int max);
```



## UnloadRandomSequence

```c
void UnloadRandomSequence(int * sequence);
```



## TakeScreenshot

```c
void TakeScreenshot(const char * fileName);
```

Misc. functions

## SetConfigFlags

```c
void SetConfigFlags(unsigned int flags);
```



## OpenURL

```c
void OpenURL(const char * url);
```



## SetTraceLogLevel

```c
void SetTraceLogLevel(int logLevel);
```

Logging system

## TraceLog

```c
void TraceLog(int logLevel, const char * text, ...);
```



## SetTraceLogCallback

```c
void SetTraceLogCallback(TraceLogCallback callback);
```



## MemAlloc

```c
void * MemAlloc(unsigned int size);
```

Memory management, using internal allocators

## MemRealloc

```c
void * MemRealloc(void * ptr, unsigned int size);
```



## MemFree

```c
void MemFree(void * ptr);
```



## LoadFileData

```c
unsigned char * LoadFileData(const char * fileName, int * dataSize);
```

File system management functions

## UnloadFileData

```c
void UnloadFileData(unsigned char * data);
```



## SaveFileData

```c
_Bool SaveFileData(const char * fileName, void * data, int dataSize);
```



## ExportDataAsCode

```c
_Bool ExportDataAsCode(const unsigned char * data, int dataSize, const char * fileName);
```



## LoadFileText

```c
char * LoadFileText(const char * fileName);
```



## UnloadFileText

```c
void UnloadFileText(char * text);
```



## SaveFileText

```c
_Bool SaveFileText(const char * fileName, const char * text);
```



## SetLoadFileDataCallback

```c
void SetLoadFileDataCallback(LoadFileDataCallback callback);
```

File access custom callbacks WARNING: Callbacks setup is intended for advanced users

## SetSaveFileDataCallback

```c
void SetSaveFileDataCallback(SaveFileDataCallback callback);
```



## SetLoadFileTextCallback

```c
void SetLoadFileTextCallback(LoadFileTextCallback callback);
```



## SetSaveFileTextCallback

```c
void SetSaveFileTextCallback(SaveFileTextCallback callback);
```



## FileRename

```c
int FileRename(const char * fileName, const char * fileRename);
```



## FileRemove

```c
int FileRemove(const char * fileName);
```



## FileCopy

```c
int FileCopy(const char * srcPath, const char * dstPath);
```



## FileMove

```c
int FileMove(const char * srcPath, const char * dstPath);
```



## FileTextReplace

```c
int FileTextReplace(const char * fileName, const char * search, const char * replacement);
```



## FileTextFindIndex

```c
int FileTextFindIndex(const char * fileName, const char * search);
```



## FileExists

```c
_Bool FileExists(const char * fileName);
```



## DirectoryExists

```c
_Bool DirectoryExists(const char * dirPath);
```



## IsFileExtension

```c
_Bool IsFileExtension(const char * fileName, const char * ext);
```



## GetFileLength

```c
int GetFileLength(const char * fileName);
```



## GetFileModTime

```c
long GetFileModTime(const char * fileName);
```



## GetFileExtension

```c
const char * GetFileExtension(const char * fileName);
```



## GetFileName

```c
const char * GetFileName(const char * filePath);
```



## GetFileNameWithoutExt

```c
const char * GetFileNameWithoutExt(const char * filePath);
```



## GetDirectoryPath

```c
const char * GetDirectoryPath(const char * filePath);
```



## GetPrevDirectoryPath

```c
const char * GetPrevDirectoryPath(const char * dirPath);
```



## GetWorkingDirectory

```c
const char * GetWorkingDirectory();
```



## GetApplicationDirectory

```c
const char * GetApplicationDirectory();
```



## MakeDirectory

```c
int MakeDirectory(const char * dirPath);
```



## ChangeDirectory

```c
_Bool ChangeDirectory(const char * dirPath);
```



## IsPathFile

```c
_Bool IsPathFile(const char * path);
```



## IsFileNameValid

```c
_Bool IsFileNameValid(const char * fileName);
```



## LoadDirectoryFiles

```c
FilePathList LoadDirectoryFiles(const char * dirPath);
```



## LoadDirectoryFilesEx

```c
FilePathList LoadDirectoryFilesEx(const char * basePath, const char * filter, _Bool scanSubdirs);
```



## UnloadDirectoryFiles

```c
void UnloadDirectoryFiles(FilePathList files);
```



## IsFileDropped

```c
_Bool IsFileDropped();
```



## LoadDroppedFiles

```c
FilePathList LoadDroppedFiles();
```



## UnloadDroppedFiles

```c
void UnloadDroppedFiles(FilePathList files);
```



## GetDirectoryFileCount

```c
unsigned int GetDirectoryFileCount(const char * dirPath);
```



## GetDirectoryFileCountEx

```c
unsigned int GetDirectoryFileCountEx(const char * basePath, const char * filter, _Bool scanSubdirs);
```



## CompressData

```c
unsigned char * CompressData(const unsigned char * data, int dataSize, int * compDataSize);
```

Compression/Encoding functionality

## DecompressData

```c
unsigned char * DecompressData(const unsigned char * compData, int compDataSize, int * dataSize);
```



## EncodeDataBase64

```c
char * EncodeDataBase64(const unsigned char * data, int dataSize, int * outputSize);
```



## DecodeDataBase64

```c
unsigned char * DecodeDataBase64(const char * text, int * outputSize);
```



## ComputeCRC32

```c
unsigned int ComputeCRC32(unsigned char * data, int dataSize);
```



## ComputeMD5

```c
unsigned int * ComputeMD5(unsigned char * data, int dataSize);
```



## ComputeSHA1

```c
unsigned int * ComputeSHA1(unsigned char * data, int dataSize);
```



## ComputeSHA256

```c
unsigned int * ComputeSHA256(unsigned char * data, int dataSize);
```



## LoadAutomationEventList

```c
AutomationEventList LoadAutomationEventList(const char * fileName);
```

Automation events functionality

## UnloadAutomationEventList

```c
void UnloadAutomationEventList(AutomationEventList list);
```



## ExportAutomationEventList

```c
_Bool ExportAutomationEventList(AutomationEventList list, const char * fileName);
```



## SetAutomationEventList

```c
void SetAutomationEventList(AutomationEventList * list);
```



## SetAutomationEventBaseFrame

```c
void SetAutomationEventBaseFrame(int frame);
```



## StartAutomationEventRecording

```c
void StartAutomationEventRecording();
```



## StopAutomationEventRecording

```c
void StopAutomationEventRecording();
```



## PlayAutomationEvent

```c
void PlayAutomationEvent(AutomationEvent event);
```



## IsKeyPressed

```c
_Bool IsKeyPressed(int key);
```

Input-related functions: keyboard

## IsKeyPressedRepeat

```c
_Bool IsKeyPressedRepeat(int key);
```



## IsKeyDown

```c
_Bool IsKeyDown(int key);
```



## IsKeyReleased

```c
_Bool IsKeyReleased(int key);
```



## IsKeyUp

```c
_Bool IsKeyUp(int key);
```



## GetKeyPressed

```c
int GetKeyPressed();
```



## GetCharPressed

```c
int GetCharPressed();
```



## GetKeyName

```c
const char * GetKeyName(int key);
```



## SetExitKey

```c
void SetExitKey(int key);
```



## IsGamepadAvailable

```c
_Bool IsGamepadAvailable(int gamepad);
```

Input-related functions: gamepads

## GetGamepadName

```c
const char * GetGamepadName(int gamepad);
```



## IsGamepadButtonPressed

```c
_Bool IsGamepadButtonPressed(int gamepad, int button);
```



## IsGamepadButtonDown

```c
_Bool IsGamepadButtonDown(int gamepad, int button);
```



## IsGamepadButtonReleased

```c
_Bool IsGamepadButtonReleased(int gamepad, int button);
```



## IsGamepadButtonUp

```c
_Bool IsGamepadButtonUp(int gamepad, int button);
```



## GetGamepadButtonPressed

```c
int GetGamepadButtonPressed();
```



## GetGamepadAxisCount

```c
int GetGamepadAxisCount(int gamepad);
```



## GetGamepadAxisMovement

```c
float GetGamepadAxisMovement(int gamepad, int axis);
```



## SetGamepadMappings

```c
int SetGamepadMappings(const char * mappings);
```



## SetGamepadVibration

```c
void SetGamepadVibration(int gamepad, float leftMotor, float rightMotor, float duration);
```



## IsMouseButtonPressed

```c
_Bool IsMouseButtonPressed(int button);
```

Input-related functions: mouse

## IsMouseButtonDown

```c
_Bool IsMouseButtonDown(int button);
```



## IsMouseButtonReleased

```c
_Bool IsMouseButtonReleased(int button);
```



## IsMouseButtonUp

```c
_Bool IsMouseButtonUp(int button);
```



## GetMouseX

```c
int GetMouseX();
```



## GetMouseY

```c
int GetMouseY();
```



## GetMousePosition

```c
Vector2 GetMousePosition();
```



## GetMouseDelta

```c
Vector2 GetMouseDelta();
```



## SetMousePosition

```c
void SetMousePosition(int x, int y);
```



## SetMouseOffset

```c
void SetMouseOffset(int offsetX, int offsetY);
```



## SetMouseScale

```c
void SetMouseScale(float scaleX, float scaleY);
```



## GetMouseWheelMove

```c
float GetMouseWheelMove();
```



## GetMouseWheelMoveV

```c
Vector2 GetMouseWheelMoveV();
```



## SetMouseCursor

```c
void SetMouseCursor(int cursor);
```



## GetTouchX

```c
int GetTouchX();
```

Input-related functions: touch

## GetTouchY

```c
int GetTouchY();
```



## GetTouchPosition

```c
Vector2 GetTouchPosition(int index);
```



## GetTouchPointId

```c
int GetTouchPointId(int index);
```



## GetTouchPointCount

```c
int GetTouchPointCount();
```



## SetGesturesEnabled

```c
void SetGesturesEnabled(unsigned int flags);
```

------------------------------------------------------------------------------------ Gestures and Touch Handling Functions (Module: rgestures) ------------------------------------------------------------------------------------

## IsGestureDetected

```c
_Bool IsGestureDetected(unsigned int gesture);
```



## GetGestureDetected

```c
int GetGestureDetected();
```



## GetGestureHoldDuration

```c
float GetGestureHoldDuration();
```



## GetGestureDragVector

```c
Vector2 GetGestureDragVector();
```



## GetGestureDragAngle

```c
float GetGestureDragAngle();
```



## GetGesturePinchVector

```c
Vector2 GetGesturePinchVector();
```



## GetGesturePinchAngle

```c
float GetGesturePinchAngle();
```



## UpdateCamera

```c
void UpdateCamera(Camera * camera, int mode);
```

------------------------------------------------------------------------------------ Camera System Functions (Module: rcamera) ------------------------------------------------------------------------------------

## UpdateCameraPro

```c
void UpdateCameraPro(Camera * camera, Vector3 movement, Vector3 rotation, float zoom);
```



## SetShapesTexture

```c
void SetShapesTexture(Texture2D texture, Rectangle source);
```

------------------------------------------------------------------------------------ Basic Shapes Drawing Functions (Module: shapes) ------------------------------------------------------------------------------------ Set texture and rectangle to be used on shapes drawing NOTE: It can be useful when using basic shapes and one single font, defining a font char white rectangle would allow drawing everything in a single draw call

## GetShapesTexture

```c
Texture2D GetShapesTexture();
```



## GetShapesTextureRectangle

```c
Rectangle GetShapesTextureRectangle();
```



## DrawPixel

```c
void DrawPixel(int posX, int posY, Color color);
```

Basic shapes drawing functions

## DrawPixelV

```c
void DrawPixelV(Vector2 position, Color color);
```



## DrawLine

```c
void DrawLine(int startPosX, int startPosY, int endPosX, int endPosY, Color color);
```



## DrawLineV

```c
void DrawLineV(Vector2 startPos, Vector2 endPos, Color color);
```



## DrawLineEx

```c
void DrawLineEx(Vector2 startPos, Vector2 endPos, float thick, Color color);
```



## DrawLineStrip

```c
void DrawLineStrip(const Vector2 * points, int pointCount, Color color);
```



## DrawLineBezier

```c
void DrawLineBezier(Vector2 startPos, Vector2 endPos, float thick, Color color);
```



## DrawLineDashed

```c
void DrawLineDashed(Vector2 startPos, Vector2 endPos, int dashSize, int spaceSize, Color color);
```



## DrawCircle

```c
void DrawCircle(int centerX, int centerY, float radius, Color color);
```



## DrawCircleV

```c
void DrawCircleV(Vector2 center, float radius, Color color);
```



## DrawCircleGradient

```c
void DrawCircleGradient(Vector2 center, float radius, Color inner, Color outer);
```



## DrawCircleSector

```c
void DrawCircleSector(Vector2 center, float radius, float startAngle, float endAngle, int segments, Color color);
```



## DrawCircleSectorLines

```c
void DrawCircleSectorLines(Vector2 center, float radius, float startAngle, float endAngle, int segments, Color color);
```



## DrawCircleLines

```c
void DrawCircleLines(int centerX, int centerY, float radius, Color color);
```



## DrawCircleLinesV

```c
void DrawCircleLinesV(Vector2 center, float radius, Color color);
```



## DrawEllipse

```c
void DrawEllipse(int centerX, int centerY, float radiusH, float radiusV, Color color);
```



## DrawEllipseV

```c
void DrawEllipseV(Vector2 center, float radiusH, float radiusV, Color color);
```



## DrawEllipseLines

```c
void DrawEllipseLines(int centerX, int centerY, float radiusH, float radiusV, Color color);
```



## DrawEllipseLinesV

```c
void DrawEllipseLinesV(Vector2 center, float radiusH, float radiusV, Color color);
```



## DrawRing

```c
void DrawRing(Vector2 center, float innerRadius, float outerRadius, float startAngle, float endAngle, int segments, Color color);
```



## DrawRingLines

```c
void DrawRingLines(Vector2 center, float innerRadius, float outerRadius, float startAngle, float endAngle, int segments, Color color);
```



## DrawRectangle

```c
void DrawRectangle(int posX, int posY, int width, int height, Color color);
```



## DrawRectangleV

```c
void DrawRectangleV(Vector2 position, Vector2 size, Color color);
```



## DrawRectangleRec

```c
void DrawRectangleRec(Rectangle rec, Color color);
```



## DrawRectanglePro

```c
void DrawRectanglePro(Rectangle rec, Vector2 origin, float rotation, Color color);
```



## DrawRectangleGradientV

```c
void DrawRectangleGradientV(int posX, int posY, int width, int height, Color top, Color bottom);
```



## DrawRectangleGradientH

```c
void DrawRectangleGradientH(int posX, int posY, int width, int height, Color left, Color right);
```



## DrawRectangleGradientEx

```c
void DrawRectangleGradientEx(Rectangle rec, Color topLeft, Color bottomLeft, Color bottomRight, Color topRight);
```



## DrawRectangleLines

```c
void DrawRectangleLines(int posX, int posY, int width, int height, Color color);
```



## DrawRectangleLinesEx

```c
void DrawRectangleLinesEx(Rectangle rec, float lineThick, Color color);
```



## DrawRectangleRounded

```c
void DrawRectangleRounded(Rectangle rec, float roundness, int segments, Color color);
```



## DrawRectangleRoundedLines

```c
void DrawRectangleRoundedLines(Rectangle rec, float roundness, int segments, Color color);
```



## DrawRectangleRoundedLinesEx

```c
void DrawRectangleRoundedLinesEx(Rectangle rec, float roundness, int segments, float lineThick, Color color);
```



## DrawTriangle

```c
void DrawTriangle(Vector2 v1, Vector2 v2, Vector2 v3, Color color);
```



## DrawTriangleLines

```c
void DrawTriangleLines(Vector2 v1, Vector2 v2, Vector2 v3, Color color);
```



## DrawTriangleFan

```c
void DrawTriangleFan(const Vector2 * points, int pointCount, Color color);
```



## DrawTriangleStrip

```c
void DrawTriangleStrip(const Vector2 * points, int pointCount, Color color);
```



## DrawPoly

```c
void DrawPoly(Vector2 center, int sides, float radius, float rotation, Color color);
```



## DrawPolyLines

```c
void DrawPolyLines(Vector2 center, int sides, float radius, float rotation, Color color);
```



## DrawPolyLinesEx

```c
void DrawPolyLinesEx(Vector2 center, int sides, float radius, float rotation, float lineThick, Color color);
```



## DrawSplineLinear

```c
void DrawSplineLinear(const Vector2 * points, int pointCount, float thick, Color color);
```

Splines drawing functions

## DrawSplineBasis

```c
void DrawSplineBasis(const Vector2 * points, int pointCount, float thick, Color color);
```



## DrawSplineCatmullRom

```c
void DrawSplineCatmullRom(const Vector2 * points, int pointCount, float thick, Color color);
```



## DrawSplineBezierQuadratic

```c
void DrawSplineBezierQuadratic(const Vector2 * points, int pointCount, float thick, Color color);
```



## DrawSplineBezierCubic

```c
void DrawSplineBezierCubic(const Vector2 * points, int pointCount, float thick, Color color);
```



## DrawSplineSegmentLinear

```c
void DrawSplineSegmentLinear(Vector2 p1, Vector2 p2, float thick, Color color);
```



## DrawSplineSegmentBasis

```c
void DrawSplineSegmentBasis(Vector2 p1, Vector2 p2, Vector2 p3, Vector2 p4, float thick, Color color);
```



## DrawSplineSegmentCatmullRom

```c
void DrawSplineSegmentCatmullRom(Vector2 p1, Vector2 p2, Vector2 p3, Vector2 p4, float thick, Color color);
```



## DrawSplineSegmentBezierQuadratic

```c
void DrawSplineSegmentBezierQuadratic(Vector2 p1, Vector2 c2, Vector2 p3, float thick, Color color);
```



## DrawSplineSegmentBezierCubic

```c
void DrawSplineSegmentBezierCubic(Vector2 p1, Vector2 c2, Vector2 c3, Vector2 p4, float thick, Color color);
```



## GetSplinePointLinear

```c
Vector2 GetSplinePointLinear(Vector2 startPos, Vector2 endPos, float t);
```

Spline segment point evaluation functions, for a given t [0.0f .. 1.0f]

## GetSplinePointBasis

```c
Vector2 GetSplinePointBasis(Vector2 p1, Vector2 p2, Vector2 p3, Vector2 p4, float t);
```



## GetSplinePointCatmullRom

```c
Vector2 GetSplinePointCatmullRom(Vector2 p1, Vector2 p2, Vector2 p3, Vector2 p4, float t);
```



## GetSplinePointBezierQuad

```c
Vector2 GetSplinePointBezierQuad(Vector2 p1, Vector2 c2, Vector2 p3, float t);
```



## GetSplinePointBezierCubic

```c
Vector2 GetSplinePointBezierCubic(Vector2 p1, Vector2 c2, Vector2 c3, Vector2 p4, float t);
```



## CheckCollisionRecs

```c
_Bool CheckCollisionRecs(Rectangle rec1, Rectangle rec2);
```

Basic shapes collision detection functions

## CheckCollisionCircles

```c
_Bool CheckCollisionCircles(Vector2 center1, float radius1, Vector2 center2, float radius2);
```



## CheckCollisionCircleRec

```c
_Bool CheckCollisionCircleRec(Vector2 center, float radius, Rectangle rec);
```



## CheckCollisionCircleLine

```c
_Bool CheckCollisionCircleLine(Vector2 center, float radius, Vector2 p1, Vector2 p2);
```



## CheckCollisionPointRec

```c
_Bool CheckCollisionPointRec(Vector2 point, Rectangle rec);
```



## CheckCollisionPointCircle

```c
_Bool CheckCollisionPointCircle(Vector2 point, Vector2 center, float radius);
```



## CheckCollisionPointTriangle

```c
_Bool CheckCollisionPointTriangle(Vector2 point, Vector2 p1, Vector2 p2, Vector2 p3);
```



## CheckCollisionPointLine

```c
_Bool CheckCollisionPointLine(Vector2 point, Vector2 p1, Vector2 p2, int threshold);
```



## CheckCollisionPointPoly

```c
_Bool CheckCollisionPointPoly(Vector2 point, const Vector2 * points, int pointCount);
```



## CheckCollisionLines

```c
_Bool CheckCollisionLines(Vector2 startPos1, Vector2 endPos1, Vector2 startPos2, Vector2 endPos2, Vector2 * collisionPoint);
```



## GetCollisionRec

```c
Rectangle GetCollisionRec(Rectangle rec1, Rectangle rec2);
```



## LoadImage

```c
Image LoadImage(const char * fileName);
```

Image loading functions NOTE: These functions do not require GPU access

## LoadImageRaw

```c
Image LoadImageRaw(const char * fileName, int width, int height, int format, int headerSize);
```



## LoadImageAnim

```c
Image LoadImageAnim(const char * fileName, int * frames);
```



## LoadImageAnimFromMemory

```c
Image LoadImageAnimFromMemory(const char * fileType, const unsigned char * fileData, int dataSize, int * frames);
```



## LoadImageFromMemory

```c
Image LoadImageFromMemory(const char * fileType, const unsigned char * fileData, int dataSize);
```



## LoadImageFromTexture

```c
Image LoadImageFromTexture(Texture2D texture);
```



## LoadImageFromScreen

```c
Image LoadImageFromScreen();
```



## IsImageValid

```c
_Bool IsImageValid(Image image);
```



## UnloadImage

```c
void UnloadImage(Image image);
```



## ExportImage

```c
_Bool ExportImage(Image image, const char * fileName);
```



## ExportImageToMemory

```c
unsigned char * ExportImageToMemory(Image image, const char * fileType, int * fileSize);
```



## ExportImageAsCode

```c
_Bool ExportImageAsCode(Image image, const char * fileName);
```



## GenImageColor

```c
Image GenImageColor(int width, int height, Color color);
```

Image generation functions

## GenImageGradientLinear

```c
Image GenImageGradientLinear(int width, int height, int direction, Color start, Color end);
```



## GenImageGradientRadial

```c
Image GenImageGradientRadial(int width, int height, float density, Color inner, Color outer);
```



## GenImageGradientSquare

```c
Image GenImageGradientSquare(int width, int height, float density, Color inner, Color outer);
```



## GenImageChecked

```c
Image GenImageChecked(int width, int height, int checksX, int checksY, Color col1, Color col2);
```



## GenImageWhiteNoise

```c
Image GenImageWhiteNoise(int width, int height, float factor);
```



## GenImagePerlinNoise

```c
Image GenImagePerlinNoise(int width, int height, int offsetX, int offsetY, float scale);
```



## GenImageCellular

```c
Image GenImageCellular(int width, int height, int tileSize);
```



## GenImageText

```c
Image GenImageText(int width, int height, const char * text);
```



## ImageCopy

```c
Image ImageCopy(Image image);
```

Image manipulation functions

## ImageFromImage

```c
Image ImageFromImage(Image image, Rectangle rec);
```



## ImageFromChannel

```c
Image ImageFromChannel(Image image, int selectedChannel);
```



## ImageText

```c
Image ImageText(const char * text, int fontSize, Color color);
```



## ImageTextEx

```c
Image ImageTextEx(Font font, const char * text, float fontSize, float spacing, Color tint);
```



## ImageFormat

```c
void ImageFormat(Image * image, int newFormat);
```



## ImageToPOT

```c
void ImageToPOT(Image * image, Color fill);
```



## ImageCrop

```c
void ImageCrop(Image * image, Rectangle crop);
```



## ImageAlphaCrop

```c
void ImageAlphaCrop(Image * image, float threshold);
```



## ImageAlphaClear

```c
void ImageAlphaClear(Image * image, Color color, float threshold);
```



## ImageAlphaMask

```c
void ImageAlphaMask(Image * image, Image alphaMask);
```



## ImageAlphaPremultiply

```c
void ImageAlphaPremultiply(Image * image);
```



## ImageBlurGaussian

```c
void ImageBlurGaussian(Image * image, int blurSize);
```



## ImageKernelConvolution

```c
void ImageKernelConvolution(Image * image, const float * kernel, int kernelSize);
```



## ImageResize

```c
void ImageResize(Image * image, int newWidth, int newHeight);
```



## ImageResizeNN

```c
void ImageResizeNN(Image * image, int newWidth, int newHeight);
```



## ImageResizeCanvas

```c
void ImageResizeCanvas(Image * image, int newWidth, int newHeight, int offsetX, int offsetY, Color fill);
```



## ImageMipmaps

```c
void ImageMipmaps(Image * image);
```



## ImageDither

```c
void ImageDither(Image * image, int rBpp, int gBpp, int bBpp, int aBpp);
```



## ImageFlipVertical

```c
void ImageFlipVertical(Image * image);
```



## ImageFlipHorizontal

```c
void ImageFlipHorizontal(Image * image);
```



## ImageRotate

```c
void ImageRotate(Image * image, int degrees);
```



## ImageRotateCW

```c
void ImageRotateCW(Image * image);
```



## ImageRotateCCW

```c
void ImageRotateCCW(Image * image);
```



## ImageColorTint

```c
void ImageColorTint(Image * image, Color color);
```



## ImageColorInvert

```c
void ImageColorInvert(Image * image);
```



## ImageColorGrayscale

```c
void ImageColorGrayscale(Image * image);
```



## ImageColorContrast

```c
void ImageColorContrast(Image * image, float contrast);
```



## ImageColorBrightness

```c
void ImageColorBrightness(Image * image, int brightness);
```



## ImageColorReplace

```c
void ImageColorReplace(Image * image, Color color, Color replace);
```



## LoadImageColors

```c
Color * LoadImageColors(Image image);
```



## LoadImagePalette

```c
Color * LoadImagePalette(Image image, int maxPaletteSize, int * colorCount);
```



## UnloadImageColors

```c
void UnloadImageColors(Color * colors);
```



## UnloadImagePalette

```c
void UnloadImagePalette(Color * colors);
```



## GetImageAlphaBorder

```c
Rectangle GetImageAlphaBorder(Image image, float threshold);
```



## GetImageColor

```c
Color GetImageColor(Image image, int x, int y);
```



## ImageClearBackground

```c
void ImageClearBackground(Image * dst, Color color);
```

Image drawing functions NOTE: Image software-rendering functions (CPU)

## ImageDrawPixel

```c
void ImageDrawPixel(Image * dst, int posX, int posY, Color color);
```



## ImageDrawPixelV

```c
void ImageDrawPixelV(Image * dst, Vector2 position, Color color);
```



## ImageDrawLine

```c
void ImageDrawLine(Image * dst, int startPosX, int startPosY, int endPosX, int endPosY, Color color);
```



## ImageDrawLineV

```c
void ImageDrawLineV(Image * dst, Vector2 start, Vector2 end, Color color);
```



## ImageDrawLineEx

```c
void ImageDrawLineEx(Image * dst, Vector2 start, Vector2 end, int thick, Color color);
```



## ImageDrawCircle

```c
void ImageDrawCircle(Image * dst, int centerX, int centerY, int radius, Color color);
```



## ImageDrawCircleV

```c
void ImageDrawCircleV(Image * dst, Vector2 center, int radius, Color color);
```



## ImageDrawCircleLines

```c
void ImageDrawCircleLines(Image * dst, int centerX, int centerY, int radius, Color color);
```



## ImageDrawCircleLinesV

```c
void ImageDrawCircleLinesV(Image * dst, Vector2 center, int radius, Color color);
```



## ImageDrawRectangle

```c
void ImageDrawRectangle(Image * dst, int posX, int posY, int width, int height, Color color);
```



## ImageDrawRectangleV

```c
void ImageDrawRectangleV(Image * dst, Vector2 position, Vector2 size, Color color);
```



## ImageDrawRectangleRec

```c
void ImageDrawRectangleRec(Image * dst, Rectangle rec, Color color);
```



## ImageDrawRectangleLines

```c
void ImageDrawRectangleLines(Image * dst, Rectangle rec, int thick, Color color);
```



## ImageDrawTriangle

```c
void ImageDrawTriangle(Image * dst, Vector2 v1, Vector2 v2, Vector2 v3, Color color);
```



## ImageDrawTriangleEx

```c
void ImageDrawTriangleEx(Image * dst, Vector2 v1, Vector2 v2, Vector2 v3, Color c1, Color c2, Color c3);
```



## ImageDrawTriangleLines

```c
void ImageDrawTriangleLines(Image * dst, Vector2 v1, Vector2 v2, Vector2 v3, Color color);
```



## ImageDrawTriangleFan

```c
void ImageDrawTriangleFan(Image * dst, const Vector2 * points, int pointCount, Color color);
```



## ImageDrawTriangleStrip

```c
void ImageDrawTriangleStrip(Image * dst, const Vector2 * points, int pointCount, Color color);
```



## ImageDraw

```c
void ImageDraw(Image * dst, Image src, Rectangle srcRec, Rectangle dstRec, Color tint);
```



## ImageDrawText

```c
void ImageDrawText(Image * dst, const char * text, int posX, int posY, int fontSize, Color color);
```



## ImageDrawTextEx

```c
void ImageDrawTextEx(Image * dst, Font font, const char * text, Vector2 position, float fontSize, float spacing, Color tint);
```



## LoadTexture

```c
Texture2D LoadTexture(const char * fileName);
```

Texture loading functions NOTE: These functions require GPU access

## LoadTextureFromImage

```c
Texture2D LoadTextureFromImage(Image image);
```



## LoadTextureCubemap

```c
TextureCubemap LoadTextureCubemap(Image image, int layout);
```



## LoadRenderTexture

```c
RenderTexture2D LoadRenderTexture(int width, int height);
```



## IsTextureValid

```c
_Bool IsTextureValid(Texture2D texture);
```



## UnloadTexture

```c
void UnloadTexture(Texture2D texture);
```



## IsRenderTextureValid

```c
_Bool IsRenderTextureValid(RenderTexture2D target);
```



## UnloadRenderTexture

```c
void UnloadRenderTexture(RenderTexture2D target);
```



## UpdateTexture

```c
void UpdateTexture(Texture2D texture, const void * pixels);
```



## UpdateTextureRec

```c
void UpdateTextureRec(Texture2D texture, Rectangle rec, const void * pixels);
```



## GenTextureMipmaps

```c
void GenTextureMipmaps(Texture2D * texture);
```

Texture configuration functions

## SetTextureFilter

```c
void SetTextureFilter(Texture2D texture, int filter);
```



## SetTextureWrap

```c
void SetTextureWrap(Texture2D texture, int wrap);
```



## DrawTexture

```c
void DrawTexture(Texture2D texture, int posX, int posY, Color tint);
```

Texture drawing functions

## DrawTextureV

```c
void DrawTextureV(Texture2D texture, Vector2 position, Color tint);
```



## DrawTextureEx

```c
void DrawTextureEx(Texture2D texture, Vector2 position, float rotation, float scale, Color tint);
```



## DrawTextureRec

```c
void DrawTextureRec(Texture2D texture, Rectangle source, Vector2 position, Color tint);
```



## DrawTexturePro

```c
void DrawTexturePro(Texture2D texture, Rectangle source, Rectangle dest, Vector2 origin, float rotation, Color tint);
```



## DrawTextureNPatch

```c
void DrawTextureNPatch(Texture2D texture, NPatchInfo nPatchInfo, Rectangle dest, Vector2 origin, float rotation, Color tint);
```



## ColorIsEqual

```c
_Bool ColorIsEqual(Color col1, Color col2);
```

Color/pixel related functions

## Fade

```c
Color Fade(Color color, float alpha);
```



## ColorToInt

```c
int ColorToInt(Color color);
```



## ColorNormalize

```c
Vector4 ColorNormalize(Color color);
```



## ColorFromNormalized

```c
Color ColorFromNormalized(Vector4 normalized);
```



## ColorToHSV

```c
Vector3 ColorToHSV(Color color);
```



## ColorFromHSV

```c
Color ColorFromHSV(float hue, float saturation, float value);
```



## ColorTint

```c
Color ColorTint(Color color, Color tint);
```



## ColorBrightness

```c
Color ColorBrightness(Color color, float factor);
```



## ColorContrast

```c
Color ColorContrast(Color color, float contrast);
```



## ColorAlpha

```c
Color ColorAlpha(Color color, float alpha);
```



## ColorAlphaBlend

```c
Color ColorAlphaBlend(Color dst, Color src, Color tint);
```



## ColorLerp

```c
Color ColorLerp(Color color1, Color color2, float factor);
```



## GetColor

```c
Color GetColor(unsigned int hexValue);
```



## GetPixelColor

```c
Color GetPixelColor(void * srcPtr, int format);
```



## SetPixelColor

```c
void SetPixelColor(void * dstPtr, Color color, int format);
```



## GetPixelDataSize

```c
int GetPixelDataSize(int width, int height, int format);
```



## GetFontDefault

```c
Font GetFontDefault();
```

Font loading/unloading functions

## LoadFont

```c
Font LoadFont(const char * fileName);
```



## LoadFontEx

```c
Font LoadFontEx(const char * fileName, int fontSize, const int * codepoints, int codepointCount);
```



## LoadFontFromImage

```c
Font LoadFontFromImage(Image image, Color key, int firstChar);
```



## LoadFontFromMemory

```c
Font LoadFontFromMemory(const char * fileType, const unsigned char * fileData, int dataSize, int fontSize, const int * codepoints, int codepointCount);
```



## IsFontValid

```c
_Bool IsFontValid(Font font);
```



## LoadFontData

```c
GlyphInfo * LoadFontData(const unsigned char * fileData, int dataSize, int fontSize, const int * codepoints, int codepointCount, int type, int * glyphCount);
```



## GenImageFontAtlas

```c
Image GenImageFontAtlas(const GlyphInfo * glyphs, Rectangle ** glyphRecs, int glyphCount, int fontSize, int padding, int packMethod);
```



## UnloadFontData

```c
void UnloadFontData(GlyphInfo * glyphs, int glyphCount);
```



## UnloadFont

```c
void UnloadFont(Font font);
```



## ExportFontAsCode

```c
_Bool ExportFontAsCode(Font font, const char * fileName);
```



## DrawFPS

```c
void DrawFPS(int posX, int posY);
```

Text drawing functions

## DrawText

```c
void DrawText(const char * text, int posX, int posY, int fontSize, Color color);
```



## DrawTextEx

```c
void DrawTextEx(Font font, const char * text, Vector2 position, float fontSize, float spacing, Color tint);
```



## DrawTextPro

```c
void DrawTextPro(Font font, const char * text, Vector2 position, Vector2 origin, float rotation, float fontSize, float spacing, Color tint);
```



## DrawTextCodepoint

```c
void DrawTextCodepoint(Font font, int codepoint, Vector2 position, float fontSize, Color tint);
```



## DrawTextCodepoints

```c
void DrawTextCodepoints(Font font, const int * codepoints, int codepointCount, Vector2 position, float fontSize, float spacing, Color tint);
```



## SetTextLineSpacing

```c
void SetTextLineSpacing(int spacing);
```

Text font info functions

## MeasureText

```c
int MeasureText(const char * text, int fontSize);
```



## MeasureTextEx

```c
Vector2 MeasureTextEx(Font font, const char * text, float fontSize, float spacing);
```



## MeasureTextCodepoints

```c
Vector2 MeasureTextCodepoints(Font font, const int * codepoints, int length, float fontSize, float spacing);
```



## GetGlyphIndex

```c
int GetGlyphIndex(Font font, int codepoint);
```



## GetGlyphInfo

```c
GlyphInfo GetGlyphInfo(Font font, int codepoint);
```



## GetGlyphAtlasRec

```c
Rectangle GetGlyphAtlasRec(Font font, int codepoint);
```



## LoadUTF8

```c
char * LoadUTF8(const int * codepoints, int length);
```

Text codepoints management functions (unicode characters)

## UnloadUTF8

```c
void UnloadUTF8(char * text);
```



## LoadCodepoints

```c
int * LoadCodepoints(const char * text, int * count);
```



## UnloadCodepoints

```c
void UnloadCodepoints(int * codepoints);
```



## GetCodepointCount

```c
int GetCodepointCount(const char * text);
```



## GetCodepoint

```c
int GetCodepoint(const char * text, int * codepointSize);
```



## GetCodepointNext

```c
int GetCodepointNext(const char * text, int * codepointSize);
```



## GetCodepointPrevious

```c
int GetCodepointPrevious(const char * text, int * codepointSize);
```



## CodepointToUTF8

```c
const char * CodepointToUTF8(int codepoint, int * utf8Size);
```



## LoadTextLines

```c
char ** LoadTextLines(const char * text, int * count);
```

Text strings management functions (no UTF-8 strings, only byte chars) WARNING 1: Most of these functions use internal static buffers[], it's recommended to store returned data on user-side for re-use WARNING 2: Some functions allocate memory internally for the returned strings, those strings must be freed by user using MemFree()

## UnloadTextLines

```c
void UnloadTextLines(char ** text, int lineCount);
```



## TextCopy

```c
int TextCopy(char * dst, const char * src);
```



## TextIsEqual

```c
_Bool TextIsEqual(const char * text1, const char * text2);
```



## TextLength

```c
unsigned int TextLength(const char * text);
```



## TextFormat

```c
const char * TextFormat(const char * text, ...);
```



## TextSubtext

```c
const char * TextSubtext(const char * text, int position, int length);
```



## TextRemoveSpaces

```c
const char * TextRemoveSpaces(const char * text);
```



## GetTextBetween

```c
char * GetTextBetween(const char * text, const char * begin, const char * end);
```



## TextReplace

```c
char * TextReplace(const char * text, const char * search, const char * replacement);
```



## TextReplaceAlloc

```c
char * TextReplaceAlloc(const char * text, const char * search, const char * replacement);
```



## TextReplaceBetween

```c
char * TextReplaceBetween(const char * text, const char * begin, const char * end, const char * replacement);
```



## TextReplaceBetweenAlloc

```c
char * TextReplaceBetweenAlloc(const char * text, const char * begin, const char * end, const char * replacement);
```



## TextInsert

```c
char * TextInsert(const char * text, const char * insert, int position);
```



## TextInsertAlloc

```c
char * TextInsertAlloc(const char * text, const char * insert, int position);
```



## TextJoin

```c
char * TextJoin(char ** textList, int count, const char * delimiter);
```



## TextSplit

```c
char ** TextSplit(const char * text, char delimiter, int * count);
```



## TextAppend

```c
void TextAppend(char * text, const char * append, int * position);
```



## TextFindIndex

```c
int TextFindIndex(const char * text, const char * search);
```



## TextToUpper

```c
char * TextToUpper(const char * text);
```



## TextToLower

```c
char * TextToLower(const char * text);
```



## TextToPascal

```c
char * TextToPascal(const char * text);
```



## TextToSnake

```c
char * TextToSnake(const char * text);
```



## TextToCamel

```c
char * TextToCamel(const char * text);
```



## TextToInteger

```c
int TextToInteger(const char * text);
```



## TextToFloat

```c
float TextToFloat(const char * text);
```



## DrawLine3D

```c
void DrawLine3D(Vector3 startPos, Vector3 endPos, Color color);
```

Basic geometric 3D shapes drawing functions

## DrawPoint3D

```c
void DrawPoint3D(Vector3 position, Color color);
```



## DrawCircle3D

```c
void DrawCircle3D(Vector3 center, float radius, Vector3 rotationAxis, float rotationAngle, Color color);
```



## DrawTriangle3D

```c
void DrawTriangle3D(Vector3 v1, Vector3 v2, Vector3 v3, Color color);
```



## DrawTriangleStrip3D

```c
void DrawTriangleStrip3D(const Vector3 * points, int pointCount, Color color);
```



## DrawCube

```c
void DrawCube(Vector3 position, float width, float height, float length, Color color);
```



## DrawCubeV

```c
void DrawCubeV(Vector3 position, Vector3 size, Color color);
```



## DrawCubeWires

```c
void DrawCubeWires(Vector3 position, float width, float height, float length, Color color);
```



## DrawCubeWiresV

```c
void DrawCubeWiresV(Vector3 position, Vector3 size, Color color);
```



## DrawSphere

```c
void DrawSphere(Vector3 centerPos, float radius, Color color);
```



## DrawSphereEx

```c
void DrawSphereEx(Vector3 centerPos, float radius, int rings, int slices, Color color);
```



## DrawSphereWires

```c
void DrawSphereWires(Vector3 centerPos, float radius, int rings, int slices, Color color);
```



## DrawCylinder

```c
void DrawCylinder(Vector3 position, float radiusTop, float radiusBottom, float height, int slices, Color color);
```



## DrawCylinderEx

```c
void DrawCylinderEx(Vector3 startPos, Vector3 endPos, float startRadius, float endRadius, int sides, Color color);
```



## DrawCylinderWires

```c
void DrawCylinderWires(Vector3 position, float radiusTop, float radiusBottom, float height, int slices, Color color);
```



## DrawCylinderWiresEx

```c
void DrawCylinderWiresEx(Vector3 startPos, Vector3 endPos, float startRadius, float endRadius, int sides, Color color);
```



## DrawCapsule

```c
void DrawCapsule(Vector3 startPos, Vector3 endPos, float radius, int slices, int rings, Color color);
```



## DrawCapsuleWires

```c
void DrawCapsuleWires(Vector3 startPos, Vector3 endPos, float radius, int slices, int rings, Color color);
```



## DrawPlane

```c
void DrawPlane(Vector3 centerPos, Vector2 size, Color color);
```



## DrawRay

```c
void DrawRay(Ray ray, Color color);
```



## DrawGrid

```c
void DrawGrid(int slices, float spacing);
```



## LoadModel

```c
Model LoadModel(const char * fileName);
```

Model management functions

## LoadModelFromMesh

```c
Model LoadModelFromMesh(Mesh mesh);
```



## IsModelValid

```c
_Bool IsModelValid(Model model);
```



## UnloadModel

```c
void UnloadModel(Model model);
```



## GetModelBoundingBox

```c
BoundingBox GetModelBoundingBox(Model model);
```



## DrawModel

```c
void DrawModel(Model model, Vector3 position, float scale, Color tint);
```

Model drawing functions

## DrawModelEx

```c
void DrawModelEx(Model model, Vector3 position, Vector3 rotationAxis, float rotationAngle, Vector3 scale, Color tint);
```



## DrawModelWires

```c
void DrawModelWires(Model model, Vector3 position, float scale, Color tint);
```



## DrawModelWiresEx

```c
void DrawModelWiresEx(Model model, Vector3 position, Vector3 rotationAxis, float rotationAngle, Vector3 scale, Color tint);
```



## DrawBoundingBox

```c
void DrawBoundingBox(BoundingBox box, Color color);
```



## DrawBillboard

```c
void DrawBillboard(Camera camera, Texture2D texture, Vector3 position, float scale, Color tint);
```



## DrawBillboardRec

```c
void DrawBillboardRec(Camera camera, Texture2D texture, Rectangle source, Vector3 position, Vector2 size, Color tint);
```



## DrawBillboardPro

```c
void DrawBillboardPro(Camera camera, Texture2D texture, Rectangle source, Vector3 position, Vector3 up, Vector2 size, Vector2 origin, float rotation, Color tint);
```



## UploadMesh

```c
void UploadMesh(Mesh * mesh, _Bool dynamic);
```

Mesh management functions

## UpdateMeshBuffer

```c
void UpdateMeshBuffer(Mesh mesh, int index, const void * data, int dataSize, int offset);
```



## UnloadMesh

```c
void UnloadMesh(Mesh mesh);
```



## DrawMesh

```c
void DrawMesh(Mesh mesh, Material material, Matrix transform);
```



## DrawMeshInstanced

```c
void DrawMeshInstanced(Mesh mesh, Material material, const Matrix * transforms, int instances);
```



## GetMeshBoundingBox

```c
BoundingBox GetMeshBoundingBox(Mesh mesh);
```



## GenMeshTangents

```c
void GenMeshTangents(Mesh * mesh);
```



## ExportMesh

```c
_Bool ExportMesh(Mesh mesh, const char * fileName);
```



## ExportMeshAsCode

```c
_Bool ExportMeshAsCode(Mesh mesh, const char * fileName);
```



## GenMeshPoly

```c
Mesh GenMeshPoly(int sides, float radius);
```

Mesh generation functions

## GenMeshPlane

```c
Mesh GenMeshPlane(float width, float length, int resX, int resZ);
```



## GenMeshCube

```c
Mesh GenMeshCube(float width, float height, float length);
```



## GenMeshSphere

```c
Mesh GenMeshSphere(float radius, int rings, int slices);
```



## GenMeshHemiSphere

```c
Mesh GenMeshHemiSphere(float radius, int rings, int slices);
```



## GenMeshCylinder

```c
Mesh GenMeshCylinder(float radius, float height, int slices);
```



## GenMeshCone

```c
Mesh GenMeshCone(float radius, float height, int slices);
```



## GenMeshTorus

```c
Mesh GenMeshTorus(float radius, float size, int radSeg, int sides);
```



## GenMeshKnot

```c
Mesh GenMeshKnot(float radius, float size, int radSeg, int sides);
```



## GenMeshHeightmap

```c
Mesh GenMeshHeightmap(Image heightmap, Vector3 size);
```



## GenMeshCubicmap

```c
Mesh GenMeshCubicmap(Image cubicmap, Vector3 cubeSize);
```



## LoadMaterials

```c
Material * LoadMaterials(const char * fileName, int * materialCount);
```

Material loading/unloading functions

## LoadMaterialDefault

```c
Material LoadMaterialDefault();
```



## IsMaterialValid

```c
_Bool IsMaterialValid(Material material);
```



## UnloadMaterial

```c
void UnloadMaterial(Material material);
```



## SetMaterialTexture

```c
void SetMaterialTexture(Material * material, int mapType, Texture2D texture);
```



## SetModelMeshMaterial

```c
void SetModelMeshMaterial(Model * model, int meshId, int materialId);
```



## LoadModelAnimations

```c
ModelAnimation * LoadModelAnimations(const char * fileName, int * animCount);
```

Model animations loading/unloading functions

## UpdateModelAnimation

```c
void UpdateModelAnimation(Model model, ModelAnimation anim, float frame);
```



## UpdateModelAnimationEx

```c
void UpdateModelAnimationEx(Model model, ModelAnimation animA, float frameA, ModelAnimation animB, float frameB, float blend);
```



## UnloadModelAnimations

```c
void UnloadModelAnimations(ModelAnimation * animations, int animCount);
```



## IsModelAnimationValid

```c
_Bool IsModelAnimationValid(Model model, ModelAnimation anim);
```



## CheckCollisionSpheres

```c
_Bool CheckCollisionSpheres(Vector3 center1, float radius1, Vector3 center2, float radius2);
```

Collision detection functions

## CheckCollisionBoxes

```c
_Bool CheckCollisionBoxes(BoundingBox box1, BoundingBox box2);
```



## CheckCollisionBoxSphere

```c
_Bool CheckCollisionBoxSphere(BoundingBox box, Vector3 center, float radius);
```



## GetRayCollisionSphere

```c
RayCollision GetRayCollisionSphere(Ray ray, Vector3 center, float radius);
```



## GetRayCollisionBox

```c
RayCollision GetRayCollisionBox(Ray ray, BoundingBox box);
```



## GetRayCollisionMesh

```c
RayCollision GetRayCollisionMesh(Ray ray, Mesh mesh, Matrix transform);
```



## GetRayCollisionTriangle

```c
RayCollision GetRayCollisionTriangle(Ray ray, Vector3 p1, Vector3 p2, Vector3 p3);
```



## GetRayCollisionQuad

```c
RayCollision GetRayCollisionQuad(Ray ray, Vector3 p1, Vector3 p2, Vector3 p3, Vector3 p4);
```



## InitAudioDevice

```c
void InitAudioDevice();
```

Audio device management functions

## CloseAudioDevice

```c
void CloseAudioDevice();
```



## IsAudioDeviceReady

```c
_Bool IsAudioDeviceReady();
```



## SetMasterVolume

```c
void SetMasterVolume(float volume);
```



## GetMasterVolume

```c
float GetMasterVolume();
```



## LoadWave

```c
Wave LoadWave(const char * fileName);
```

Wave/Sound loading/unloading functions

## LoadWaveFromMemory

```c
Wave LoadWaveFromMemory(const char * fileType, const unsigned char * fileData, int dataSize);
```



## IsWaveValid

```c
_Bool IsWaveValid(Wave wave);
```



## LoadSound

```c
Sound LoadSound(const char * fileName);
```



## LoadSoundFromWave

```c
Sound LoadSoundFromWave(Wave wave);
```



## LoadSoundAlias

```c
Sound LoadSoundAlias(Sound source);
```



## IsSoundValid

```c
_Bool IsSoundValid(Sound sound);
```



## UpdateSound

```c
void UpdateSound(Sound sound, const void * data, int sampleCount);
```



## UnloadWave

```c
void UnloadWave(Wave wave);
```



## UnloadSound

```c
void UnloadSound(Sound sound);
```



## UnloadSoundAlias

```c
void UnloadSoundAlias(Sound alias);
```



## ExportWave

```c
_Bool ExportWave(Wave wave, const char * fileName);
```



## ExportWaveAsCode

```c
_Bool ExportWaveAsCode(Wave wave, const char * fileName);
```



## PlaySound

```c
void PlaySound(Sound sound);
```

Wave/Sound management functions

## StopSound

```c
void StopSound(Sound sound);
```



## PauseSound

```c
void PauseSound(Sound sound);
```



## ResumeSound

```c
void ResumeSound(Sound sound);
```



## IsSoundPlaying

```c
_Bool IsSoundPlaying(Sound sound);
```



## SetSoundVolume

```c
void SetSoundVolume(Sound sound, float volume);
```



## SetSoundPitch

```c
void SetSoundPitch(Sound sound, float pitch);
```



## SetSoundPan

```c
void SetSoundPan(Sound sound, float pan);
```



## WaveCopy

```c
Wave WaveCopy(Wave wave);
```



## WaveCrop

```c
void WaveCrop(Wave * wave, int initFrame, int finalFrame);
```



## WaveFormat

```c
void WaveFormat(Wave * wave, int sampleRate, int sampleSize, int channels);
```



## LoadWaveSamples

```c
float * LoadWaveSamples(Wave wave);
```



## UnloadWaveSamples

```c
void UnloadWaveSamples(float * samples);
```



## LoadMusicStream

```c
Music LoadMusicStream(const char * fileName);
```

Music management functions

## LoadMusicStreamFromMemory

```c
Music LoadMusicStreamFromMemory(const char * fileType, const unsigned char * data, int dataSize);
```



## IsMusicValid

```c
_Bool IsMusicValid(Music music);
```



## UnloadMusicStream

```c
void UnloadMusicStream(Music music);
```



## PlayMusicStream

```c
void PlayMusicStream(Music music);
```



## IsMusicStreamPlaying

```c
_Bool IsMusicStreamPlaying(Music music);
```



## UpdateMusicStream

```c
void UpdateMusicStream(Music music);
```



## StopMusicStream

```c
void StopMusicStream(Music music);
```



## PauseMusicStream

```c
void PauseMusicStream(Music music);
```



## ResumeMusicStream

```c
void ResumeMusicStream(Music music);
```



## SeekMusicStream

```c
void SeekMusicStream(Music music, float position);
```



## SetMusicVolume

```c
void SetMusicVolume(Music music, float volume);
```



## SetMusicPitch

```c
void SetMusicPitch(Music music, float pitch);
```



## SetMusicPan

```c
void SetMusicPan(Music music, float pan);
```



## GetMusicTimeLength

```c
float GetMusicTimeLength(Music music);
```



## GetMusicTimePlayed

```c
float GetMusicTimePlayed(Music music);
```



## LoadAudioStream

```c
AudioStream LoadAudioStream(unsigned int sampleRate, unsigned int sampleSize, unsigned int channels);
```

AudioStream management functions

## IsAudioStreamValid

```c
_Bool IsAudioStreamValid(AudioStream stream);
```



## UnloadAudioStream

```c
void UnloadAudioStream(AudioStream stream);
```



## UpdateAudioStream

```c
void UpdateAudioStream(AudioStream stream, const void * data, int frameCount);
```



## IsAudioStreamProcessed

```c
_Bool IsAudioStreamProcessed(AudioStream stream);
```



## PlayAudioStream

```c
void PlayAudioStream(AudioStream stream);
```



## PauseAudioStream

```c
void PauseAudioStream(AudioStream stream);
```



## ResumeAudioStream

```c
void ResumeAudioStream(AudioStream stream);
```



## IsAudioStreamPlaying

```c
_Bool IsAudioStreamPlaying(AudioStream stream);
```



## StopAudioStream

```c
void StopAudioStream(AudioStream stream);
```



## SetAudioStreamVolume

```c
void SetAudioStreamVolume(AudioStream stream, float volume);
```



## SetAudioStreamPitch

```c
void SetAudioStreamPitch(AudioStream stream, float pitch);
```



## SetAudioStreamPan

```c
void SetAudioStreamPan(AudioStream stream, float pan);
```



## SetAudioStreamBufferSizeDefault

```c
void SetAudioStreamBufferSizeDefault(int size);
```



## SetAudioStreamCallback

```c
void SetAudioStreamCallback(AudioStream stream, AudioCallback callback);
```



## AttachAudioStreamProcessor

```c
void AttachAudioStreamProcessor(AudioStream stream, AudioCallback processor);
```



## DetachAudioStreamProcessor

```c
void DetachAudioStreamProcessor(AudioStream stream, AudioCallback processor);
```



## AttachAudioMixedProcessor

```c
void AttachAudioMixedProcessor(AudioCallback processor);
```



## DetachAudioMixedProcessor

```c
void DetachAudioMixedProcessor(AudioCallback processor);
```



