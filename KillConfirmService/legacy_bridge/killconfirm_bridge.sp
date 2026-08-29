#pragma semicolon 1
#pragma newdecls required

#include <sourcemod>

public Plugin myinfo =
{
    name = "KillConfirm Legacy Bridge",
    author = "KillConfirmGameBar",
    description = "Exposes Legacy death and bot-control events to the local overlay service",
    version = "0.2.0",
    url = ""
};

char g_LogPath[PLATFORM_MAX_PATH];
int g_ControlledBotUserId[MAXPLAYERS + 1];
int g_ControlledKillCount[MAXPLAYERS + 1];

public void OnPluginStart()
{
    BuildPath(Path_SM, g_LogPath, sizeof(g_LogPath), "logs/killconfirm_bridge.log");
    HookEvent("player_death", Event_PlayerDeath, EventHookMode_Post);
    HookEventEx("bot_takeover", Event_BotTakeover, EventHookMode_Post);
    HookEvent("round_start", Event_RoundStart, EventHookMode_PostNoCopy);
    WriteBridgeLine("READY|version=0.2.0");
}

public void OnMapStart()
{
    ClearAllControlState();
}

public void OnClientDisconnect(int client)
{
    ClearControlState(client);
}

public void Event_RoundStart(Event event, const char[] name, bool dontBroadcast)
{
    ClearAllControlState();
}

public void Event_PlayerDeath(Event event, const char[] name, bool dontBroadcast)
{
    int attackerUserId = event.GetInt("attacker");
    int victimUserId = event.GetInt("userid");
    int attacker = GetClientOfUserId(attackerUserId);
    int victim = GetClientOfUserId(victimUserId);
    bool controlled = IsUsableClient(attacker) && g_ControlledBotUserId[attacker] != 0;
    bool victimWasController = victim > 0 && victim <= MaxClients && g_ControlledBotUserId[victim] != 0;

    if (!controlled)
    {
        if (victimWasController)
        {
            ClearControlState(victim);
        }
        return;
    }

    g_ControlledKillCount[attacker]++;

    char attackerName[MAX_NAME_LENGTH], attackerAuth[64];
    char victimName[MAX_NAME_LENGTH], victimAuth[64];
    char weapon[64];
    DescribeClient(attacker, attackerName, sizeof(attackerName), attackerAuth, sizeof(attackerAuth));
    DescribeClient(victim, victimName, sizeof(victimName), victimAuth, sizeof(victimAuth));
    event.GetString("weapon", weapon, sizeof(weapon));
    Sanitize(attackerName, sizeof(attackerName));
    Sanitize(victimName, sizeof(victimName));
    Sanitize(weapon, sizeof(weapon));

    int attackerTeam = GetClientTeam(attacker);
    int enemiesAliveAfter = CountEnemiesAliveAfter(attackerTeam, victim);

    char line[896];
    FormatEx(
        line,
        sizeof(line),
        "DEATH|controlled=1|controlled_bot_uid=%d|controlled_count=%d|enemies_alive_after=%d|attacker_uid=%d|attacker_client=%d|attacker_auth=%s|attacker_name=%s|attacker_bot=%d|victim_uid=%d|victim_client=%d|victim_auth=%s|victim_name=%s|weapon=%s|headshot=%d",
        g_ControlledBotUserId[attacker],
        g_ControlledKillCount[attacker],
        enemiesAliveAfter,
        attackerUserId,
        attacker,
        attackerAuth,
        attackerName,
        IsUsableClient(attacker) && IsFakeClient(attacker),
        victimUserId,
        victim,
        victimAuth,
        victimName,
        weapon,
        event.GetBool("headshot")
    );
    WriteBridgeLine(line);

    if (victimWasController)
    {
        ClearControlState(victim);
    }
}

public void Event_BotTakeover(Event event, const char[] name, bool dontBroadcast)
{
    int playerUserId = event.GetInt("userid");
    int player = GetClientOfUserId(playerUserId);
    int botUserId = event.GetInt("botid");
    if (IsUsableClient(player))
    {
        g_ControlledBotUserId[player] = botUserId;
        g_ControlledKillCount[player] = 0;
    }
    WriteControlEvent("BOT_TAKEOVER", playerUserId, botUserId, event.GetInt("index"));
}

int CountEnemiesAliveAfter(int attackerTeam, int victim)
{
    int count = 0;
    for (int client = 1; client <= MaxClients; client++)
    {
        if (client == victim || !IsUsableClient(client) || !IsPlayerAlive(client))
        {
            continue;
        }

        int team = GetClientTeam(client);
        if (team >= 2 && team != attackerTeam)
        {
            count++;
        }
    }
    return count;
}

void ClearAllControlState()
{
    for (int client = 1; client <= MaxClients; client++)
    {
        ClearControlState(client);
    }
}

void ClearControlState(int client)
{
    if (client <= 0 || client > MaxClients)
    {
        return;
    }
    g_ControlledBotUserId[client] = 0;
    g_ControlledKillCount[client] = 0;
}

void WriteControlEvent(const char[] kind, int playerUserId, int botUserId, int index)
{
    int player = GetClientOfUserId(playerUserId);
    int bot = GetClientOfUserId(botUserId);
    char playerName[MAX_NAME_LENGTH], playerAuth[64];
    char botName[MAX_NAME_LENGTH], botAuth[64];
    DescribeClient(player, playerName, sizeof(playerName), playerAuth, sizeof(playerAuth));
    DescribeClient(bot, botName, sizeof(botName), botAuth, sizeof(botAuth));
    Sanitize(playerName, sizeof(playerName));
    Sanitize(botName, sizeof(botName));

    char line[640];
    FormatEx(
        line,
        sizeof(line),
        "%s|player_uid=%d|player_client=%d|player_auth=%s|player_name=%s|bot_uid=%d|bot_client=%d|bot_auth=%s|bot_name=%s|index=%d",
        kind,
        playerUserId,
        player,
        playerAuth,
        playerName,
        botUserId,
        bot,
        botAuth,
        botName,
        index
    );
    WriteBridgeLine(line);
}

void DescribeClient(int client, char[] clientName, int nameLength, char[] auth, int authLength)
{
    strcopy(clientName, nameLength, "none");
    strcopy(auth, authLength, "none");
    if (!IsUsableClient(client))
    {
        return;
    }

    GetClientName(client, clientName, nameLength);
    if (!GetClientAuthId(client, AuthId_SteamID64, auth, authLength, false))
    {
        strcopy(auth, authLength, IsFakeClient(client) ? "BOT" : "unknown");
    }
}

bool IsUsableClient(int client)
{
    return client > 0 && client <= MaxClients && IsClientInGame(client);
}

void Sanitize(char[] value, int maxLength)
{
    ReplaceString(value, maxLength, "|", "/", false);
    ReplaceString(value, maxLength, "\r", " ", false);
    ReplaceString(value, maxLength, "\n", " ", false);
}

void WriteBridgeLine(const char[] line)
{
    LogToFileEx(g_LogPath, "%s", line);
}
