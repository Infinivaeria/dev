#include <X11/XKBlib.h>
#include <X11/Xlib.h>
#include <X11/Xutil.h>
#include <X11/keysym.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

/* Sends keystrokes to the window owned by a process:
 *   x11_key_driver PID wait:MS | type:TEXT | key:[ctrl+|shift+|alt+]NAME | close ... */

static Window find_window(Display *display, Window parent, unsigned long pid) {
    Atom actual_type;
    int format;
    unsigned long count, remaining;
    unsigned char *property = NULL;
    Atom pid_atom = XInternAtom(display, "_NET_WM_PID", False);
    if (XGetWindowProperty(display, parent, pid_atom, 0, 1, False, AnyPropertyType,
                           &actual_type, &format, &count, &remaining, &property) == Success) {
        int matches = property && format == 32 && count == 1 && *(unsigned long *)property == pid;
        if (property) XFree(property);
        if (matches) return parent;
    }
    Window root, ancestor, *children = NULL;
    unsigned int child_count;
    if (!XQueryTree(display, parent, &root, &ancestor, &children, &child_count)) return None;
    Window found = None;
    for (unsigned int index = 0; index < child_count && !found; index++) {
        found = find_window(display, children[index], pid);
    }
    if (children) XFree(children);
    return found;
}

static void send_key(Display *display, Window window, KeyCode code, int type, unsigned int state) {
    XKeyEvent event;
    memset(&event, 0, sizeof event);
    event.type = type;
    event.display = display;
    event.window = window;
    event.root = DefaultRootWindow(display);
    event.subwindow = None;
    event.time = CurrentTime;
    event.x = event.y = event.x_root = event.y_root = 1;
    event.same_screen = True;
    event.keycode = code;
    event.state = state;
    XSendEvent(display, window, True, type == KeyPress ? KeyPressMask : KeyReleaseMask, (XEvent *)&event);
    XFlush(display);
}

/* Held long enough that the application sees the key down in at least one frame. */
static void tap(Display *display, Window window, KeySym symbol, unsigned int modifiers) {
    KeyCode code = XKeysymToKeycode(display, symbol);
    if (!code) {
        fprintf(stderr, "no keycode for keysym %lx\n", (unsigned long)symbol);
        exit(6);
    }
    KeySym held[3];
    unsigned int masks[3];
    int count = 0;
    if (modifiers & ControlMask) { held[count] = XK_Control_L; masks[count++] = ControlMask; }
    if (modifiers & ShiftMask) { held[count] = XK_Shift_L; masks[count++] = ShiftMask; }
    if (modifiers & Mod1Mask) { held[count] = XK_Alt_L; masks[count++] = Mod1Mask; }
    unsigned int state = 0;
    for (int index = 0; index < count; index++) {
        send_key(display, window, XKeysymToKeycode(display, held[index]), KeyPress, state);
        state |= masks[index];
        usleep(40000);
    }
    send_key(display, window, code, KeyPress, state);
    usleep(50000);
    send_key(display, window, code, KeyRelease, state);
    usleep(40000);
    for (int index = count - 1; index >= 0; index--) {
        state &= ~masks[index];
        send_key(display, window, XKeysymToKeycode(display, held[index]), KeyRelease, state | masks[index]);
        usleep(30000);
    }
}

static void send_motion(Display *display, Window window, int x, int y, unsigned int state) {
    XMotionEvent event;
    memset(&event, 0, sizeof event);
    event.type = MotionNotify;
    event.display = display;
    event.window = window;
    event.root = DefaultRootWindow(display);
    event.subwindow = None;
    event.time = CurrentTime;
    event.x = event.x_root = x;
    event.y = event.y_root = y;
    event.state = state;
    event.is_hint = NotifyNormal;
    event.same_screen = True;
    XSendEvent(display, window, True, PointerMotionMask, (XEvent *)&event);
    XFlush(display);
}

static void send_button(Display *display, Window window, int x, int y, int type) {
    XButtonEvent event;
    memset(&event, 0, sizeof event);
    event.type = type;
    event.display = display;
    event.window = window;
    event.root = DefaultRootWindow(display);
    event.subwindow = None;
    event.time = CurrentTime;
    event.x = event.x_root = x;
    event.y = event.y_root = y;
    event.state = type == ButtonRelease ? Button1Mask : 0;
    event.button = Button1;
    event.same_screen = True;
    XSendEvent(display, window, True, type == ButtonPress ? ButtonPressMask : ButtonReleaseMask,
               (XEvent *)&event);
    XFlush(display);
}

static void drag(Display *display, Window window, int x1, int y1, int x2, int y2) {
    send_motion(display, window, x1, y1, 0);
    usleep(60000);
    send_button(display, window, x1, y1, ButtonPress);
    usleep(80000);
    for (int step = 1; step <= 12; step++) {
        send_motion(display, window, x1 + (x2 - x1) * step / 12, y1 + (y2 - y1) * step / 12, Button1Mask);
        usleep(25000);
    }
    usleep(60000);
    send_button(display, window, x2, y2, ButtonRelease);
    usleep(60000);
}

static int capture(Display *display, Window window, const char *path) {
    XWindowAttributes attributes;
    XGetWindowAttributes(display, window, &attributes);
    XImage *image = XGetImage(display, window, 0, 0, attributes.width, attributes.height, AllPlanes, ZPixmap);
    if (!image) return 8;
    FILE *output = fopen(path, "wb");
    if (!output) return 9;
    fprintf(output, "P6\n%d %d\n255\n", attributes.width, attributes.height);
    for (int row = 0; row < attributes.height; row++) {
        for (int column = 0; column < attributes.width; column++) {
            unsigned long pixel = XGetPixel(image, column, row);
            unsigned char rgb[] = {(pixel & image->red_mask) >> 16, (pixel & image->green_mask) >> 8,
                                   pixel & image->blue_mask};
            fwrite(rgb, 1, 3, output);
        }
    }
    fclose(output);
    XDestroyImage(image);
    return 0;
}

static void type_character(Display *display, Window window, unsigned char c) {
    KeySym symbol = c == '\n' ? XK_Return : (KeySym)c;
    KeyCode code = XKeysymToKeycode(display, symbol);
    if (!code) return;
    unsigned int modifiers = XkbKeycodeToKeysym(display, code, 0, 0) == symbol ? 0 : ShiftMask;
    tap(display, window, symbol, modifiers);
}

static int ignore_error(Display *display, XErrorEvent *event) {
    (void)display;
    (void)event;
    return 0; /* the window may close while its last key is still being released */
}

int main(int argc, char **argv) {
    if (argc < 3) return 2;
    XSetErrorHandler(ignore_error);
    Display *display = XOpenDisplay(NULL);
    if (!display) return 3;
    unsigned long pid = strtoul(argv[1], NULL, 10);
    Window target = None;
    for (int attempt = 0; attempt < 200 && !target; attempt++) {
        target = find_window(display, DefaultRootWindow(display), pid);
        if (!target) usleep(50000);
    }
    if (!target) return 4;
    for (int index = 2; index < argc; index++) {
        const char *command = argv[index];
        if (strncmp(command, "wait:", 5) == 0) {
            usleep((useconds_t)atoi(command + 5) * 1000);
        } else if (strncmp(command, "type:", 5) == 0) {
            for (const char *c = command + 5; *c; c++) type_character(display, target, (unsigned char)*c);
        } else if (strncmp(command, "key:", 4) == 0) {
            const char *name = command + 4;
            unsigned int modifiers = 0;
            for (;;) {
                if (strncmp(name, "ctrl+", 5) == 0) { modifiers |= ControlMask; name += 5; }
                else if (strncmp(name, "shift+", 6) == 0) { modifiers |= ShiftMask; name += 6; }
                else if (strncmp(name, "alt+", 4) == 0) { modifiers |= Mod1Mask; name += 4; }
                else break;
            }
            KeySym symbol = XStringToKeysym(name);
            if (symbol == NoSymbol) return 5;
            tap(display, target, symbol, modifiers);
        } else if (strncmp(command, "move:", 5) == 0) {
            int x, y;
            if (sscanf(command + 5, "%d,%d", &x, &y) != 2) return 10;
            send_motion(display, target, x, y, 0);
            usleep(60000);
        } else if (strncmp(command, "click:", 6) == 0) {
            int x, y;
            if (sscanf(command + 6, "%d,%d", &x, &y) != 2) return 10;
            send_motion(display, target, x, y, 0);
            usleep(80000);
            send_button(display, target, x, y, ButtonPress);
            usleep(90000);
            send_button(display, target, x, y, ButtonRelease);
            usleep(60000);
        } else if (strncmp(command, "drag:", 5) == 0) {
            int x1, y1, x2, y2;
            if (sscanf(command + 5, "%d,%d,%d,%d", &x1, &y1, &x2, &y2) != 4) return 10;
            drag(display, target, x1, y1, x2, y2);
        } else if (strncmp(command, "capture:", 8) == 0) {
            int status = capture(display, target, command + 8);
            if (status) return status;
        } else if (strcmp(command, "close") == 0) {
            XEvent event;
            memset(&event, 0, sizeof event);
            event.xclient.type = ClientMessage;
            event.xclient.display = display;
            event.xclient.window = target;
            event.xclient.message_type = XInternAtom(display, "WM_PROTOCOLS", False);
            event.xclient.format = 32;
            event.xclient.data.l[0] = XInternAtom(display, "WM_DELETE_WINDOW", False);
            event.xclient.data.l[1] = CurrentTime;
            XSendEvent(display, target, False, NoEventMask, &event);
            XFlush(display);
        } else {
            return 7;
        }
    }
    XCloseDisplay(display);
    return 0;
}
