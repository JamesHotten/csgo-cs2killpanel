using System;
using Windows.Storage;

namespace KillConfirmGameBar.Services
{
    internal static class SettingsConfigurationService
    {
        internal const int CurrentVersion = 2;
        internal const string VersionKey = "SettingsConfigurationVersion";
        internal const string SpectatedPlayerEffectsKey = "SpectatedPlayerEffectsEnabled";
        internal const string ReplayEffectsKey = "ReplayEffectsEnabled";
        internal const string ControlledBotEffectsKey = "ControlledBotEffectsEnabled";

        private const string LegacySpectatedEffectsKey = "SpectatedKillEffectsEnabled";
        private const string CloseBehaviorKey = "CloseWindowBehavior";

        internal static void EnsureMigrated()
        {
            var values = ApplicationData.Current.LocalSettings.Values;
            int version = ReadVersion(values[VersionKey]);

            if (version < 1)
            {
                string closeMode = values[CloseBehaviorKey] as string;
                values[CloseBehaviorKey] = string.Equals(
                    closeMode,
                    CloseBehaviorSettingsStore.ExitMode,
                    StringComparison.OrdinalIgnoreCase)
                    ? CloseBehaviorSettingsStore.ExitMode
                    : CloseBehaviorSettingsStore.KeepRunningMode;
            }

            if (version < 2)
            {
                bool legacyEnabled = ReadBoolean(values[LegacySpectatedEffectsKey], true);
                SetBooleanIfMissing(values, SpectatedPlayerEffectsKey, legacyEnabled);
                SetBooleanIfMissing(values, ReplayEffectsKey, legacyEnabled);
                SetBooleanIfMissing(values, ControlledBotEffectsKey, legacyEnabled);
            }

            values[VersionKey] = CurrentVersion;
        }

        internal static bool ReadBooleanSetting(string key, bool fallback = true)
        {
            EnsureMigrated();
            return ReadBoolean(ApplicationData.Current.LocalSettings.Values[key], fallback);
        }

        private static int ReadVersion(object value)
        {
            if (value is int number)
            {
                return Math.Max(0, number);
            }

            return value is string text && int.TryParse(text, out number)
                ? Math.Max(0, number)
                : 0;
        }

        private static bool ReadBoolean(object value, bool fallback)
        {
            if (value is bool enabled)
            {
                return enabled;
            }

            return value is string text && bool.TryParse(text, out enabled)
                ? enabled
                : fallback;
        }

        private static void SetBooleanIfMissing(
            Windows.Foundation.Collections.IPropertySet values,
            string key,
            bool value)
        {
            if (!values.ContainsKey(key))
            {
                values[key] = value;
            }
        }
    }
}
