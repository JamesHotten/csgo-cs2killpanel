using System;
using System.Collections.Generic;
using System.Globalization;
using System.Threading.Tasks;
using Windows.Data.Json;
using Windows.Storage;

namespace KillConfirmGameBar.Services
{
    internal static class SettingsBackupService
    {
        private const int BackupSchemaVersion = 2;

        internal static async Task ExportAsync(StorageFile file)
        {
            SettingsConfigurationService.EnsureMigrated();
            var settings = new JsonObject();
            foreach (KeyValuePair<string, object> pair in ApplicationData.Current.LocalSettings.Values)
            {
                if (!ShouldTransferKey(pair.Key) || !TrySerializeValue(pair.Value, out JsonObject entry))
                {
                    continue;
                }

                settings[pair.Key] = entry;
            }

            var root = new JsonObject
            {
                ["schema_version"] = JsonValue.CreateNumberValue(BackupSchemaVersion),
                ["settings_version"] = JsonValue.CreateNumberValue(
                    SettingsConfigurationService.CurrentVersion),
                ["exported_at_utc"] = JsonValue.CreateStringValue(
                    DateTimeOffset.UtcNow.ToString("O", CultureInfo.InvariantCulture)),
                ["settings"] = settings
            };
            await FileIO.WriteTextAsync(file, root.Stringify());
        }

        internal static async Task<int> ImportAsync(StorageFile file)
        {
            string text = await FileIO.ReadTextAsync(file);
            JsonObject root = JsonObject.Parse(text);
            int schemaVersion = (int)root.GetNamedNumber("schema_version", 1);
            if (schemaVersion < 1 || schemaVersion > BackupSchemaVersion)
            {
                throw new InvalidOperationException(
                    "Unsupported settings backup schema: " + schemaVersion);
            }

            JsonObject settings = root.GetNamedObject("settings", null);
            if (settings == null)
            {
                throw new InvalidOperationException("The settings backup does not contain a settings object.");
            }

            var imported = new Dictionary<string, object>(StringComparer.Ordinal);
            foreach (KeyValuePair<string, IJsonValue> pair in settings)
            {
                if (!ShouldTransferKey(pair.Key))
                {
                    continue;
                }

                object value = schemaVersion == 1
                    ? DeserializeLegacyValue(pair.Value)
                    : DeserializeTypedValue(pair.Value);
                if (value != null)
                {
                    imported[pair.Key] = value;
                }
            }

            if (imported.Count == 0)
            {
                throw new InvalidOperationException("The settings backup contains no supported values.");
            }

            var target = ApplicationData.Current.LocalSettings.Values;
            foreach (KeyValuePair<string, object> pair in imported)
            {
                target[pair.Key] = pair.Value;
            }
            SettingsConfigurationService.EnsureMigrated();
            return imported.Count;
        }

        private static bool ShouldTransferKey(string key)
        {
            if (string.IsNullOrWhiteSpace(key))
            {
                return false;
            }

            return key.IndexOf("AccessToken", StringComparison.OrdinalIgnoreCase) < 0
                && !string.Equals(key, "CsInstallFolderToken", StringComparison.OrdinalIgnoreCase)
                && !string.Equals(key, "CsInstallFolderPath", StringComparison.OrdinalIgnoreCase);
        }

        private static bool TrySerializeValue(object value, out JsonObject entry)
        {
            entry = null;
            string type;
            string serialized;
            switch (value)
            {
                case string text:
                    type = "string";
                    serialized = text;
                    break;
                case bool boolean:
                    type = "bool";
                    serialized = boolean ? "true" : "false";
                    break;
                case int intValue:
                    type = "int32";
                    serialized = intValue.ToString(CultureInfo.InvariantCulture);
                    break;
                case long longValue:
                    type = "int64";
                    serialized = longValue.ToString(CultureInfo.InvariantCulture);
                    break;
                case double doubleValue:
                    type = "double";
                    serialized = doubleValue.ToString("R", CultureInfo.InvariantCulture);
                    break;
                case float floatValue:
                    type = "double";
                    serialized = floatValue.ToString("R", CultureInfo.InvariantCulture);
                    break;
                default:
                    return false;
            }

            entry = new JsonObject
            {
                ["type"] = JsonValue.CreateStringValue(type),
                ["value"] = JsonValue.CreateStringValue(serialized ?? string.Empty)
            };
            return true;
        }

        private static object DeserializeTypedValue(IJsonValue json)
        {
            if (json.ValueType != JsonValueType.Object)
            {
                throw new InvalidOperationException("A settings entry is not a typed object.");
            }

            JsonObject entry = json.GetObject();
            string type = entry.GetNamedString("type", string.Empty);
            string value = entry.GetNamedString("value", string.Empty);
            switch (type)
            {
                case "string":
                    return value;
                case "bool":
                    return bool.TryParse(value, out bool boolean)
                        ? (object)boolean
                        : throw new InvalidOperationException("Invalid Boolean setting value.");
                case "int32":
                    return int.TryParse(value, NumberStyles.Integer, CultureInfo.InvariantCulture, out int int32)
                        ? (object)int32
                        : throw new InvalidOperationException("Invalid Int32 setting value.");
                case "int64":
                    return long.TryParse(value, NumberStyles.Integer, CultureInfo.InvariantCulture, out long int64)
                        ? (object)int64
                        : throw new InvalidOperationException("Invalid Int64 setting value.");
                case "double":
                    return double.TryParse(value, NumberStyles.Float, CultureInfo.InvariantCulture, out double number)
                        ? (object)number
                        : throw new InvalidOperationException("Invalid Double setting value.");
                default:
                    throw new InvalidOperationException("Unsupported setting value type: " + type);
            }
        }

        private static object DeserializeLegacyValue(IJsonValue json)
        {
            switch (json.ValueType)
            {
                case JsonValueType.String:
                    return json.GetString();
                case JsonValueType.Boolean:
                    return json.GetBoolean();
                case JsonValueType.Number:
                    double number = json.GetNumber();
                    return Math.Abs(number % 1) < double.Epsilon
                        && number >= int.MinValue
                        && number <= int.MaxValue
                        ? (object)(int)number
                        : number;
                default:
                    throw new InvalidOperationException("Unsupported legacy setting value.");
            }
        }
    }
}
