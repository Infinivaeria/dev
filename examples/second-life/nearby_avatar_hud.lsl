// Region-wide avatar HUD for Second Life (LSL / Mono).
// Put this script in a prim, then attach the prim to a HUD position.
// Click the HUD to advance to the next page. Only the owner can operate it.
// llGetAgentList can return at most 100 agents from the current region.

float SCAN_INTERVAL = 5.0;    // Seconds; clamped to at least 1.0.
integer ROWS_PER_PAGE = 3;
vector TEXT_COLOR = <0.3, 1.0, 0.5>;

// Strided list: distance, name.
list gAvatars;
integer gPage;
integer gAtAgentLimit;
string gScanError;

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
    if (gScanError != "")
    {
        llSetText("Avatar scan unavailable\n" + gScanError,
            TEXT_COLOR, 1.0);
        return;
    }

    if (count == 0)
    {
        llSetText("Nearby avatars\nNo other avatars in this region",
            TEXT_COLOR, 1.0);
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

    if (gAtAgentLimit) text += "\n100-agent limit reached";
    if (pages > 1) text += "\nClick for next page";
    llSetText(text, TEXT_COLOR, 1.0);
}

scanRegion()
{
    list agents = llGetAgentList(AGENT_LIST_REGION, []);
    integer agentCount = llGetListLength(agents);
    gAvatars = [];
    gAtAgentLimit = (agentCount >= 100);
    gScanError = "";

    if (agentCount > 0
        && llGetListEntryType(agents, 0) == TYPE_STRING)
    {
        gScanError = llList2String(agents, 0);
        showPage();
        return;
    }

    vector origin = llGetPos();
    list ownerPosition = llGetObjectDetails(llGetOwner(), [OBJECT_POS]);
    if (llGetListLength(ownerPosition) > 0)
        origin = llList2Vector(ownerPosition, 0);

    integer i;
    for (i = 0; i < agentCount; ++i)
    {
        key avatar = llList2Key(agents, i);
        if (avatar != NULL_KEY && avatar != llGetOwner())
        {
            // llGetAgentSize also filters stale presence records ("ghosts").
            if (llGetAgentSize(avatar) != ZERO_VECTOR)
            {
                list details = llGetObjectDetails(avatar, [OBJECT_POS]);
                if (llGetListLength(details) > 0)
                {
                    gAvatars += [
                        llVecDist(origin, llList2Vector(details, 0)),
                        shortName(llKey2Name(avatar))
                    ];
                }
            }
        }
    }

    gAvatars = llListSort(gAvatars, 2, TRUE);
    if (gPage >= ((llGetListLength(gAvatars) / 2
        + ROWS_PER_PAGE - 1) / ROWS_PER_PAGE))
        gPage = 0;
    showPage();
}

startScanner()
{
    llSetTimerEvent(0.0);
    gAvatars = [];
    gPage = 0;
    gAtAgentLimit = FALSE;
    gScanError = "";

    if (llGetAttached() == 0)
    {
        llSetText("Attach this prim as a HUD\nto scan nearby avatars",
            TEXT_COLOR, 1.0);
        return;
    }

    if (SCAN_INTERVAL < 1.0) SCAN_INTERVAL = 1.0;
    // Three rows leave room for multibyte names and the agent-limit notice.
    if (ROWS_PER_PAGE < 1) ROWS_PER_PAGE = 1;
    if (ROWS_PER_PAGE > 3) ROWS_PER_PAGE = 3;

    llSetText("Nearby avatars\nScanning...", TEXT_COLOR, 1.0);
    scanRegion();
    llSetTimerEvent(SCAN_INTERVAL);
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
            llSetTimerEvent(0.0);
            llSetText("", TEXT_COLOR, 0.0);
        }
    }

    changed(integer change)
    {
        if (change & CHANGED_OWNER) llResetScript();
        if (change & (CHANGED_REGION | CHANGED_TELEPORT)) startScanner();
    }

    timer()
    {
        scanRegion();
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
