using System;
using System.Threading.Tasks;
using KillConfirmGameBar.Controls.GameStyles;
using KillConfirmGameBar.Services;
using Windows.Storage;
using Windows.Storage.Pickers;
using Windows.Storage.Provider;
using Windows.UI.Xaml;
using Windows.UI.Xaml.Controls;
using Windows.UI.Xaml.Media;
using Windows.UI.Popups;
using System.Collections.Generic;

namespace KillConfirmGameBar.Controls.Settings
{
    public sealed partial class GeneralSettingsOptionsPanel : UserControl
    {
        private bool _suppressSpectatedKillEffectsEvents;
        private bool _suppressDanmaku6657Events;
        private bool _suppressAutoCloseOnGameExitEvents;
        private bool _suppressInterruptPreviousKillAudioEvents;
        private bool _suppressStreakGainEvents = true;
        private readonly DispatcherTimer _streakGainSyncTimer = new DispatcherTimer();

        public GeneralSettingsOptionsPanel()
        {
            InitializeComponent();
            _streakGainSyncTimer.Interval = TimeSpan.FromMilliseconds(250);
            _streakGainSyncTimer.Tick += OnStreakGainSyncTimerTick;
            ApplyLanguage();
            RefreshSettings();
            Loaded += OnLoaded;
        }

        private void OnLoaded(object sender, RoutedEventArgs e)
        {
            RefreshSettings();
            ApplyTheme(GameThemePalette.Current);
        }

        internal void ApplyLanguage()
        {
            SpectatedKillEffectsLabelText.Text =
                LocalizationManager.Text("SpectatedKillEffectsLabel");
            SpectatedKillEffectsHintText.Text =
                LocalizationManager.Text("SpectatedKillEffectsHint");
            SpectatedKillEffectsToggle.OffContent = LocalizationManager.Text("Off");
            SpectatedKillEffectsToggle.OnContent = LocalizationManager.Text("On");
            bool isChinese = LocalizationManager.Current == UiLanguage.SimplifiedChinese;
            ReplayEffectsLabelText.Text = isChinese ? "回放击杀特效" : "Replay kill effects";
            ReplayEffectsHintText.Text = isChinese
                ? "在 GOTV、Demo 或主视角回放中显示当前观察目标的新击杀"
                : "Show new kills for the current target in GOTV, demos, and replay feeds.";
            ControlledBotEffectsLabelText.Text = isChinese ? "接管机器人击杀特效" : "Controlled-bot kill effects";
            ControlledBotEffectsHintText.Text = isChinese
                ? "通过 CS2 本地日志桥或 Legacy SourceMod 桥显示接管击杀"
                : "Show takeover kills detected by the CS2 log or Legacy SourceMod bridge.";
            ReplayEffectsToggle.OffContent = LocalizationManager.Text("Off");
            ReplayEffectsToggle.OnContent = LocalizationManager.Text("On");
            ControlledBotEffectsToggle.OffContent = LocalizationManager.Text("Off");
            ControlledBotEffectsToggle.OnContent = LocalizationManager.Text("On");
            SettingsBackupLabelText.Text = isChinese ? "设置导出与导入" : "Export and import settings";
            SettingsBackupHintText.Text = isChinese
                ? "备份位置、尺寸、颜色、风格与音效选项；不会导出令牌和安装路径"
                : "Back up placement, size, colors, styles, and audio; tokens and install paths are excluded.";
            ExportSettingsButton.Content = isChinese ? "导出" : "Export";
            ImportSettingsButton.Content = isChinese ? "导入" : "Import";
            Danmaku6657LabelText.Text = isChinese ? "游戏事件弹幕" : "Game Event Danmaku";
            Danmaku6657HintText.Text = isChinese
                ? "根据战斗、目标与回合事件显示 5–7 条分类弹幕，单条最长 5 秒"
                : "Shows 5–7 categorized comments for combat, objective, and round events; each completes within 5 seconds.";
            Danmaku6657TestButton.Content = isChinese ? "测试所选事件" : "Test selected event";
            Danmaku6657Toggle.OffContent = LocalizationManager.Text("Off");
            Danmaku6657Toggle.OnContent = LocalizationManager.Text("On");
            BombAudioPanel?.ApplyLanguage();
            AutoCloseOnGameExitLabelText.Text =
                LocalizationManager.Text("AutoCloseOnGameExitLabel");
            AutoCloseOnGameExitHintText.Text =
                LocalizationManager.Text("AutoCloseOnGameExitHint");
            AutoCloseOnGameExitToggle.OffContent = LocalizationManager.Text("Off");
            AutoCloseOnGameExitToggle.OnContent = LocalizationManager.Text("On");
            InterruptPreviousKillAudioLabelText.Text =
                LocalizationManager.Text("InterruptPreviousKillAudioLabel");
            InterruptPreviousKillAudioHintText.Text =
                LocalizationManager.Text("InterruptPreviousKillAudioHint");
            InterruptPreviousKillAudioToggle.OffContent = LocalizationManager.Text("Off");
            InterruptPreviousKillAudioToggle.OnContent = LocalizationManager.Text("On");
            StreakGainLabelText.Text = isChinese ? "连杀音量递增" : "Streak volume gain";
            StreakGainHintText.Text = isChinese
                ? "对所有游戏和语音包生效，连杀越多音量越高"
                : "Applies to every game and voice pack; higher streaks play louder.";
            StreakGainStepLabelText.Text = isChinese ? "每次击杀增加" : "Gain per kill";
            StreakGainMaximumLabelText.Text = isChinese ? "最高音量" : "Maximum volume";
            StreakGainToggle.OffContent = LocalizationManager.Text("Off");
            StreakGainToggle.OnContent = LocalizationManager.Text("On");
        }

        internal void ApplyTheme(GameThemePalette theme)
        {
            if (theme == null)
            {
                return;
            }

            SpectatedKillEffectsHintText.Foreground = new SolidColorBrush(theme.MutedText);
            ReplayEffectsHintText.Foreground = new SolidColorBrush(theme.MutedText);
            ControlledBotEffectsHintText.Foreground = new SolidColorBrush(theme.MutedText);
            SettingsBackupHintText.Foreground = new SolidColorBrush(theme.MutedText);
            Danmaku6657HintText.Foreground = new SolidColorBrush(theme.MutedText);
            DanmakuSettingsOptions?.ApplyTheme(theme);
            BombAudioPanel?.ApplyTheme(theme);
            AutoCloseOnGameExitHintText.Foreground = new SolidColorBrush(theme.MutedText);
            InterruptPreviousKillAudioHintText.Foreground = new SolidColorBrush(theme.MutedText);
            StreakGainHintText.Foreground = new SolidColorBrush(theme.MutedText);
            AdvancedEffectsPanelSupport.ApplySoftenedTree(this, theme);
        }

        internal void RefreshSettings()
        {
            SelectSpectatedKillEffects();
            SelectDanmaku6657();
            BombAudioPanel?.RefreshSettings();
            SelectAutoCloseOnGameExit();
            SelectInterruptPreviousKillAudio();
            SelectStreakGainSettings();
        }

        private void SelectDanmaku6657()
        {
            _suppressDanmaku6657Events = true;
            try
            {
                Danmaku6657Toggle.IsOn = KillConfirmGameBar.Danmaku.DanmakuSettingsStore.IsEnabled;
                DanmakuSettingsOptions.Visibility = Danmaku6657Toggle.IsOn ? Visibility.Visible : Visibility.Collapsed;
                DanmakuSettingsOptions.RefreshSettings();
            }
            finally
            {
                _suppressDanmaku6657Events = false;
            }
        }

        private void OnDanmaku6657Toggled(object sender, RoutedEventArgs e)
        {
            if (_suppressDanmaku6657Events)
            {
                return;
            }

            KillConfirmGameBar.Danmaku.DanmakuSettingsStore.IsEnabled = Danmaku6657Toggle.IsOn;
            DanmakuSettingsOptions.Visibility = Danmaku6657Toggle.IsOn ? Visibility.Visible : Visibility.Collapsed;
        }

        private void OnDanmaku6657TestClick(object sender, RoutedEventArgs e)
        {
            DanmakuSettingsOptions?.TestSelectedEvent();
        }

        private void SelectSpectatedKillEffects()
        {
            _suppressSpectatedKillEffectsEvents = true;
            try
            {
                SpectatedKillEffectsToggle.IsOn =
                    SharedStreakSettingsStore.LoadSpectatedPlayerEffects();
                ReplayEffectsToggle.IsOn = SharedStreakSettingsStore.LoadReplayEffects();
                ControlledBotEffectsToggle.IsOn = SharedStreakSettingsStore.LoadControlledBotEffects();
            }
            finally
            {
                _suppressSpectatedKillEffectsEvents = false;
            }
        }

        private async void OnSpectatedKillEffectsToggled(object sender, RoutedEventArgs e)
        {
            await SaveObservedEffectsAsync();
        }

        private async void OnObservedEffectsToggled(object sender, RoutedEventArgs e)
        {
            await SaveObservedEffectsAsync();
        }

        private async Task SaveObservedEffectsAsync()
        {
            if (_suppressSpectatedKillEffectsEvents)
            {
                return;
            }

            SharedStreakSettingsStore.SaveObservedEffects(
                SpectatedKillEffectsToggle.IsOn,
                ReplayEffectsToggle.IsOn,
                ControlledBotEffectsToggle.IsOn);
            try
            {
                await SharedStreakSettingsStore.SyncObservedEffectsAsync();
            }
            catch (Exception ex)
            {
                // The local value is authoritative and will be synchronized at service startup.
                App.Log("Set spectated player kill effects failed: " + ex);
            }
        }

        private async void OnExportSettingsClick(object sender, RoutedEventArgs e)
        {
            var picker = new FileSavePicker
            {
                SuggestedStartLocation = PickerLocationId.DocumentsLibrary,
                SuggestedFileName = "KillConfirmSettings-" + DateTimeOffset.Now.ToString("yyyyMMdd-HHmmss")
            };
            picker.FileTypeChoices.Add("JSON", new List<string> { ".json" });
            StorageFile file = await picker.PickSaveFileAsync();
            if (file == null) return;
            try
            {
                CachedFileManager.DeferUpdates(file);
                await SettingsBackupService.ExportAsync(file);
                if (await CachedFileManager.CompleteUpdatesAsync(file) != FileUpdateStatus.Complete)
                {
                    throw new InvalidOperationException("Windows could not finalize the settings file.");
                }
                await ShowSettingsMessageAsync(LocalizationManager.Current == UiLanguage.SimplifiedChinese
                    ? "设置已导出。" : "Settings exported.");
            }
            catch (Exception ex)
            {
                App.Log("Export settings failed: " + ex);
                await ShowSettingsMessageAsync(ex.Message);
            }
        }

        private async void OnImportSettingsClick(object sender, RoutedEventArgs e)
        {
            var picker = new FileOpenPicker
            {
                SuggestedStartLocation = PickerLocationId.DocumentsLibrary,
                ViewMode = PickerViewMode.List
            };
            picker.FileTypeFilter.Add(".json");
            StorageFile file = await picker.PickSingleFileAsync();
            if (file == null) return;
            try
            {
                int count = await SettingsBackupService.ImportAsync(file);
                RefreshSettings();
                await SharedStreakSettingsStore.SyncObservedEffectsAsync();
                await ShowSettingsMessageAsync(LocalizationManager.Current == UiLanguage.SimplifiedChinese
                    ? $"已导入 {count} 项设置；重新打开小组件后全部生效。"
                    : $"Imported {count} settings. Reopen the widget to apply every value.");
            }
            catch (Exception ex)
            {
                App.Log("Import settings failed: " + ex);
                await ShowSettingsMessageAsync(ex.Message);
            }
        }

        private static async Task ShowSettingsMessageAsync(string message)
        {
            try { await new MessageDialog(message, "Kill Confirm Overlay").ShowAsync(); }
            catch { }
        }

        private void SelectAutoCloseOnGameExit()
        {
            _suppressAutoCloseOnGameExitEvents = true;
            try
            {
                AutoCloseOnGameExitToggle.IsOn = AutoCloseOnGameExitSettingsStore.Load();
            }
            finally
            {
                _suppressAutoCloseOnGameExitEvents = false;
            }
        }

        private void OnAutoCloseOnGameExitToggled(object sender, RoutedEventArgs e)
        {
            if (_suppressAutoCloseOnGameExitEvents)
            {
                return;
            }

            AutoCloseOnGameExitSettingsStore.Save(AutoCloseOnGameExitToggle.IsOn);
        }

        private void SelectInterruptPreviousKillAudio()
        {
            _suppressInterruptPreviousKillAudioEvents = true;
            try
            {
                InterruptPreviousKillAudioToggle.IsOn = InterruptPreviousKillAudioSettingsStore.Load();
            }
            finally
            {
                _suppressInterruptPreviousKillAudioEvents = false;
            }
        }

        private async void OnInterruptPreviousKillAudioToggled(object sender, RoutedEventArgs e)
        {
            if (_suppressInterruptPreviousKillAudioEvents)
            {
                return;
            }

            InterruptPreviousKillAudioSettingsStore.Save(InterruptPreviousKillAudioToggle.IsOn);
            try
            {
                await InterruptPreviousKillAudioSettingsStore.SyncAsync();
            }
            catch (Exception ex)
            {
                // The local value is authoritative and will be synchronized at service startup.
                App.Log("Set interrupt previous kill audio failed: " + ex);
            }
        }

        private void SelectStreakGainSettings()
        {
            _suppressStreakGainEvents = true;
            try
            {
                StreakGainSettingsValues settings = StreakGainSettingsStore.Load();
                StreakGainToggle.IsOn = settings.Enabled;
                StreakGainStepSlider.Value = settings.StepPercent;
                StreakGainMaximumSlider.Value = settings.MaximumPercent;
                SetStreakGainControlsEnabled(settings.Enabled);
                UpdateStreakGainValueTexts();
            }
            finally
            {
                _suppressStreakGainEvents = false;
            }
        }

        private async void OnStreakGainToggled(object sender, RoutedEventArgs e)
        {
            if (_suppressStreakGainEvents) return;
            SetStreakGainControlsEnabled(StreakGainToggle.IsOn);
            SaveStreakGainSettings();
            await SyncStreakGainSettingsAsync();
        }

        private void OnStreakGainValueChanged(
            object sender,
            Windows.UI.Xaml.Controls.Primitives.RangeBaseValueChangedEventArgs e)
        {
            if (StreakGainStepSlider == null || StreakGainMaximumSlider == null) return;
            UpdateStreakGainValueTexts();
            if (_suppressStreakGainEvents) return;
            SaveStreakGainSettings();
            _streakGainSyncTimer.Stop();
            _streakGainSyncTimer.Start();
        }

        private async void OnStreakGainSyncTimerTick(object sender, object e)
        {
            _streakGainSyncTimer.Stop();
            await SyncStreakGainSettingsAsync();
        }

        private void SaveStreakGainSettings()
        {
            StreakGainSettingsStore.Save(
                StreakGainToggle.IsOn,
                StreakGainStepSlider.Value,
                StreakGainMaximumSlider.Value);
        }

        private void UpdateStreakGainValueTexts()
        {
            StreakGainStepValueText.Text = Math.Round(StreakGainStepSlider.Value) + "%";
            StreakGainMaximumValueText.Text = Math.Round(StreakGainMaximumSlider.Value) + "%";
        }

        private void SetStreakGainControlsEnabled(bool enabled)
        {
            StreakGainStepSlider.IsEnabled = enabled;
            StreakGainMaximumSlider.IsEnabled = enabled;
        }

        private static async Task SyncStreakGainSettingsAsync()
        {
            try
            {
                await StreakGainSettingsStore.SyncAsync();
            }
            catch (Exception ex)
            {
                App.Log("Set streak gain settings failed: " + ex);
            }
        }

    }
}
