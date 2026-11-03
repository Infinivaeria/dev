#include <X11/Xlib.h>
#include <X11/Xutil.h>
#include <X11/keysym.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/select.h>

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

static void message(Display *display, Window target, const char *type,
                    long first, long second, long third, long fourth, long fifth) {
    XEvent event = {0};
    event.xclient.type = ClientMessage;
    event.xclient.display = display;
    event.xclient.window = target;
    event.xclient.message_type = XInternAtom(display, type, False);
    event.xclient.format = 32;
    event.xclient.data.l[0] = first;
    event.xclient.data.l[1] = second;
    event.xclient.data.l[2] = third;
    event.xclient.data.l[3] = fourth;
    event.xclient.data.l[4] = fifth;
    XSendEvent(display, target, False, NoEventMask, &event);
    XFlush(display);
}

int main(int argc, char **argv) {
    if (argc < 3) return 2;
    Display *display = XOpenDisplay(NULL);
    if (!display) return 3;
    Window target = find_window(display, DefaultRootWindow(display), strtoul(argv[1], NULL, 10));
    if (!target) return 4;
    XWindowAttributes attributes;
    XGetWindowAttributes(display, target, &attributes);
    if (strcmp(argv[2], "capture") == 0) {
        XImage *image = XGetImage(display, target, 0, 0, attributes.width, attributes.height, AllPlanes, ZPixmap);
        if (!image || argc != 4) return 5;
        FILE *output = fopen(argv[3], "wb");
        if (!output) return 6;
        fprintf(output, "P6\n%d %d\n255\n", attributes.width, attributes.height);
        for (int row = 0; row < attributes.height; row++) {
            for (int column = 0; column < attributes.width; column++) {
                unsigned long pixel = XGetPixel(image, column, row);
                unsigned char rgb[] = {(pixel & image->red_mask) >> 16,
                                       (pixel & image->green_mask) >> 8,
                                       pixel & image->blue_mask};
                fwrite(rgb, 1, 3, output);
            }
        }
        fclose(output);
        XDestroyImage(image);
    } else if (strcmp(argv[2], "close") == 0) {
        message(display, target, "WM_PROTOCOLS", XInternAtom(display, "WM_DELETE_WINDOW", False), CurrentTime, 0, 0, 0);
    } else if (strcmp(argv[2], "drop") == 0 && argc == 5) {
        FILE *input = fopen(argv[3], "rb");
        if (!input) return 7;
        char uris[16384];
        size_t length = fread(uris, 1, sizeof(uris), input);
        fclose(input);
        Window source = XCreateSimpleWindow(display, DefaultRootWindow(display), 0, 0, 1, 1, 0, 0, 0);
        Atom selection = XInternAtom(display, "XdndSelection", False);
        Atom uri_type = XInternAtom(display, "text/uri-list", False);
        Atom copy = XInternAtom(display, "XdndActionCopy", False);
        XSetSelectionOwner(display, selection, source, CurrentTime);
        message(display, target, "XdndEnter", source, 5L << 24, uri_type, 0, 0);
        int root_x, root_y;
        Window child;
        int offset = strcmp(argv[4], "row") == 0 ? -174 : -80;
        XTranslateCoordinates(display, target, DefaultRootWindow(display), attributes.width / 2,
                              attributes.height / 2 + offset, &root_x, &root_y, &child);
        message(display, target, "XdndPosition", source, 0, (root_x << 16) | (root_y & 0xffff), CurrentTime, copy);
        int dropped = 0, finished = 0;
        for (int attempt = 0; attempt < 100 && !finished; attempt++) {
            while (XPending(display)) {
                XEvent event;
                XNextEvent(display, &event);
                if (event.type == ClientMessage && event.xclient.message_type == XInternAtom(display, "XdndStatus", False) && !dropped) {
                    if (!(event.xclient.data.l[1] & 1)) return 8;
                    message(display, target, "XdndDrop", source, 0, CurrentTime, 0, 0);
                    dropped = 1;
                } else if (event.type == SelectionRequest) {
                    XSelectionRequestEvent *request = &event.xselectionrequest;
                    Atom property = request->property ? request->property : request->target;
                    XChangeProperty(display, request->requestor, property, uri_type, 8, PropModeReplace, (unsigned char *)uris, length);
                    XEvent response = {0};
                    response.xselection.type = SelectionNotify;
                    response.xselection.display = display;
                    response.xselection.requestor = request->requestor;
                    response.xselection.selection = request->selection;
                    response.xselection.target = request->target;
                    response.xselection.property = property;
                    response.xselection.time = request->time;
                    XSendEvent(display, request->requestor, False, NoEventMask, &response);
                    XFlush(display);
                } else if (event.type == ClientMessage && event.xclient.message_type == XInternAtom(display, "XdndFinished", False)) {
                    finished = 1;
                }
            }
            if (!finished) {
                fd_set descriptors;
                FD_ZERO(&descriptors);
                FD_SET(ConnectionNumber(display), &descriptors);
                struct timeval timeout = {0, 100000};
                select(ConnectionNumber(display) + 1, &descriptors, NULL, NULL, &timeout);
            }
        }
        XDestroyWindow(display, source);
        if (!finished) return 9;
    } else return 2;
    XCloseDisplay(display);
    return 0;
}