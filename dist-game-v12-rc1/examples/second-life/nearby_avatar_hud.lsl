// Nearby avatar HUD for Second Life (LSL / Mono).
// Put this script in a prim, then attach the prim to a HUD position.
// Click the HUD to advance to the next page. Only the owner can operate it.
// Sensors are limited to 96 meters and 16 results per scan, in this region.

float SCAN_RANGE = 96.0;       // Meters; clamped to 0.1 .. 96.0.
float SCAN_INTERVAL = 2.0;    // Seconds; clamped to at least 1.0.
integer ROWS_PER_PAGE = 3;
vector TEXT_COLOR = <0.3, 1.0, 0.5>;

// Strided list: distance, name. Sensor results arrive nearest first.
list gAvatars;
integer gPage;
integer gAtSensorLimit;

string shortName(string name)
{
    // Base64 length <= 44 guarantees at most 33 UTF-8 bytes, even for
    // non-ASCII names. Keep the full HUD comfortably below llSetText's
    // 254-byte limit, without cutting a UTF-8 byte sequence in half.
    if (llStringLength(llStringToBase64(name)) <= 44)
        return name;

    while (llStringLength(llStringToBase64(name + "...")) > 44)
        name = llDeleteSubString(name, -1, -1);
    return name + "...";
}

showPage()
{
    integer count = llGetListLength(gAvatars) / 2;
    if (count == 0)
    {
        llSetText("Nearby avatars\nNone detected within "
            + (string)SCAN_RANGE + " m", TEXT_COLOR, 1.0);
        return;
    }

    integer pages = (count + ROWS_PER_PAGE - 1) / ROWS_PER_PAGE;
    if (gPage >= pages) gPage = 0;

    string text = "Nearby: " + (string)count
        + " | Page " + (string)(gPage + 1) + "/" + (string)pages;
    integer first = gPage * ROWS_PER_PAGE;
    integer last = first + ROWS_PER_PAGE;
    if (last > count) last = count;

    integer i;
    for (i = first; i < last; ++i)
    {
        integer meters = llRound(llList2Float(gAvatars, i * 2));
        text += "\n" + llList2String(gAvatars, i * 2 + 1)
            + " - " + (string)meters + " m";
    }

    if (gAtSensorLimit) text += "\n16-result limit; more may be nearby";
    if (pages > 1) text += "\nClick for next page";
    llSetText(text, TEXT_COLOR, 1.0);
}

startScanner()
{
    llSensorRemove();
    gAvatars = [];
    gPage = 0;
    gAtSensorLimit = FALSE;

    if (llGetAttached() == 0)
    {
        llSetText("Attach this prim as a HUD\nto scan nearby avatars",
            TEXT_COLOR, 1.0);
        return;
    }

    if (SCAN_RANGE < 0.1) SCAN_RANGE = 0.1;
    if (SCAN_RANGE > 96.0) SCAN_RANGE = 96.0;
    if (SCAN_INTERVAL < 1.0) SCAN_INTERVAL = 1.0;
    // Three rows leave room for multibyte names and the sensor-limit notice.
    if (ROWS_PER_PAGE < 1) ROWS_PER_PAGE = 1;
    if (ROWS_PER_PAGE > 3) ROWS_PER_PAGE = 3;

    llSetText("Nearby avatars\nScanning...", TEXT_COLOR, 1.0);
    llSensorRepeat("", NULL_KEY, AGENT, SCAN_RANGE, PI, SCAN_INTERVAL);
}

default
{
    state_entry()
    {
        startScanner();
    }

    attach(key avatar)
    {
        if (avatar != NULL_KEY) startScanner();
        else
        {
            llSensorRemove();
            llSetText("", TEXT_COLOR, 0.0);
        }
    }

    changed(integer change)
    {
        if (change & CHANGED_OWNER) llResetScript();
        if (change & (CHANGED_REGION | CHANGED_TELEPORT)) startScanner();
    }

    sensor(integer detected)
    {
        gAvatars = [];
        gAtSensorLimit = (detected >= 16);
        vector origin = llGetPos();
        integer i;
        for (i = 0; i < detected; ++i)
        {
            // Explicitly exclude the wearer even if the sensor returns them.
            if (llDetectedKey(i) != llGetOwner())
                gAvatars += [llVecDist(origin, llDetectedPos(i)),
                    shortName(llDetectedName(i))];
        }
        showPage();
    }

    no_sensor()
    {
        gAvatars = [];
        gPage = 0;
        gAtSensorLimit = FALSE;
        showPage();
    }

    touch_start(integer touches)
    {
        integer i;
        for (i = 0; i < touches; ++i)
        {
            if (llDetectedKey(i) == llGetOwner())
            {
                ++gPage;
                showPage();
            }
        }
    }
}
