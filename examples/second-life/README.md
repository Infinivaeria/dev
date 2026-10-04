# Nearby avatar HUD

[nearby_avatar_hud.lsl](./nearby_avatar_hud.lsl) is a standalone Second Life
LSL script. It displays nearby avatar names and rounded distances as floating
text on a HUD prim, nearest first, with three avatars per page.

## Install

1. In Second Life, create a small prim and open **Edit → Contents → New Script**.
2. Replace the script contents with the linked LSL file, select **Mono**, and
   save. Ensure **Running** is enabled.
3. Take the object into inventory, then choose **Attach to HUD → Center** (or
   another HUD position). Move it so the floating text sits inside the screen.
4. Wait up to two seconds for results. Click the prim to cycle through pages.

HUD attachment text is visible to the wearer. The script does not send names
to chat, other objects, or external services. Leave the prim visible so it is
easy to click.

## Settings and limits

- `SCAN_RANGE`: radius in meters, default 96; valid range 0.1–96.
- `SCAN_INTERVAL`: refresh interval in seconds, default 2; minimum 1.
- `TEXT_COLOR`: RGB vector with components between 0 and 1.
- `ROWS_PER_PAGE`: 1–3, default 3, to respect the floating-text byte limit.
- The wearer is excluded. Names come from `llDetectedName` (legacy avatar
  names); long names are shortened for display.
- Scans cover a full sphere, including avatars above or below you and behind
  walls. Distances are three-dimensional, rounded to the nearest meter.
- LSL sensors return at most 16 avatars per scan and do not cross region
  boundaries. A notice appears when the sensor returns 16 results; this HUD
  is not a complete census of crowded areas.
- Pages refresh as people move, so their membership can change between scans.
- Teleporting or crossing regions restarts the scanner and clears old results.
- Use a HUD attachment for the intended display. A normal body attachment
  exposes its floating text in-world. Scanning is disabled while rezzed.

## In-world verification

No Second Life runtime or LSL compiler is bundled with this project. Compile
the script in the viewer, then check:

1. Alone: the HUD says no avatars were detected.
2. With another avatar nearby: their name and approximate distance appear;
   the wearer's name does not.
3. With four or more nearby avatars: clicking advances pages and wraps around.
4. After everyone leaves range: old names disappear at the next scan.
5. In a crowd of at least 16 other avatars: the sensor-limit notice appears.
6. After a teleport and after detaching/reattaching: results refresh for the
   current location.
