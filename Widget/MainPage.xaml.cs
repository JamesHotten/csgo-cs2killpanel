using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Threading.Tasks;
using KillConfirmGameBar.Services;
using Windows.Media.Core;
using Windows.Media.Playback;
using Windows.Storage;
using Windows.Storage.Pickers;
using Windows.Storage.Provider;
using Windows.UI;
using Windows.UI.Popups;
using Windows.UI.Xaml;
using Windows.UI.Xaml.Controls;
using Windows.UI.Xaml.Media;
using Windows.UI.Xaml.Media.Imaging;
using KillConfirmGameBar.Helpers;

namespace KillConfirmGameBar
{
    public sealed partial class MainPage : Page
    {
        private readonly MediaPlayer _previewPlayer = new MediaPlayer();
        private bool _iconSpecExpanded;
        private bool _suppressCloseBehaviorEvents;
        private bool _suppressSpectatedKillEffectsEvents;
        private bool _showingHomePage = true;

        public MainPage()
        {
            InitializeComponent();
            ApplyLanguage();
            LoadCloseBehaviorSetting();
            LoadObservedEffectsSettings();
            GameStyleService.Changed += OnGameStyleServiceChanged;
            Loaded += OnLoaded;
            Unloaded += OnUnloaded;
        }

        private void LoadCloseBehaviorSetting()
        {
            if (CloseBehaviorSelector == null)
            {
                return;
            }

            _suppressCloseBehaviorEvents = true;
            try
            {
                string value = CloseBehaviorSettingsStore.Load();
                string targetTag = string.Equals(value, "exit", StringComparison.OrdinalIgnoreCase) ? "exit" : "tray";
                foreach (object item in CloseBehaviorSelector.Items)
                {
                    if (item is ComboBoxItem comboItem && comboItem.Tag is string tag && string.Equals(tag, targetTag, StringComparison.OrdinalIgnoreCase))
                    {
                        CloseBehaviorSelector.SelectedItem = comboItem;
                        break;
                    }
                }
            }
            finally
            {
                _suppressCloseBehaviorEvents = false;
            }
        }

        private void OnCloseBehaviorSelectionChanged(object sender, SelectionChangedEventArgs e)
        {
            if (_suppressCloseBehaviorEvents)
            {
                return;
            }

            if (CloseBehaviorSelector?.SelectedItem is ComboBoxItem selected && selected.Tag is string mode)
            {
                CloseBehaviorSettingsStore.Save(mode);
            }
        }

        private void OnGameStyleServiceChanged(object sender, GameStyleMode mode)
        {
            _ = Dispatcher.RunAsync(Windows.UI.Core.CoreDispatcherPriority.Normal, () =>
            {
                ApplyGameStyleUi();
            });
        }

        private void LoadObservedEffectsSettings()
        {
            if (SpectatedPlayerEffectsToggle == null
                || ReplayEffectsToggle == null
                || ControlledBotEffectsToggle == null)
            {
                return;
            }

            _suppressSpectatedKillEffectsEvents = true;
            try
            {
                SpectatedPlayerEffectsToggle.IsOn =
                    SharedStreakSettingsStore.LoadSpectatedPlayerEffects();
                ReplayEffectsToggle.IsOn = SharedStreakSettingsStore.LoadReplayEffects();
                ControlledBotEffectsToggle.IsOn =
                    SharedStreakSettingsStore.LoadControlledBotEffects();
            }
            finally
            {
                _suppressSpectatedKillEffectsEvents = false;
            }
        }

        private async void OnObservedEffectsToggled(object sender, RoutedEventArgs e)
        {
            if (_suppressSpectatedKillEffectsEvents)
            {
                return;
            }

            SharedStreakSettingsStore.SaveObservedEffects(
                SpectatedPlayerEffectsToggle.IsOn,
                ReplayEffectsToggle.IsOn,
                ControlledBotEffectsToggle.IsOn);
            try
            {
                await SharedStreakSettingsStore.SyncObservedEffectsAsync();
            }
            catch (Exception ex)
            {
                // The saved value is synchronized when the widget starts its service.
                App.Log("Set spectated player kill effects failed: " + ex);
            }
        }

        private async void OnExportSettingsClick(object sender, RoutedEventArgs e)
        {
            var picker = new FileSavePicker
            {
                SuggestedStartLocation = PickerLocationId.DocumentsLibrary,
                SuggestedFileName = "KillConfirmSettings-"
                    + DateTimeOffset.Now.ToString("yyyyMMdd-HHmmss")
            };
            picker.FileTypeChoices.Add("JSON", new List<string> { ".json" });
            StorageFile file = await picker.PickSaveFileAsync();
            if (file == null)
            {
                return;
            }

            try
            {
                CachedFileManager.DeferUpdates(file);
                await SettingsBackupService.ExportAsync(file);
                FileUpdateStatus status = await CachedFileManager.CompleteUpdatesAsync(file);
                if (status != FileUpdateStatus.Complete)
                {
                    throw new InvalidOperationException("Windows could not finalize the exported settings file.");
                }

                await ShowSettingsMessageAsync(
                    LocalizationManager.Current == UiLanguage.SimplifiedChinese
                        ? "设置已导出。"
                        : "Settings exported.");
            }
            catch (Exception ex)
            {
                App.Log("Export settings failed: " + ex);
                await ShowSettingsMessageAsync(
                    (LocalizationManager.Current == UiLanguage.SimplifiedChinese
                        ? "导出设置失败："
                        : "Settings export failed: ") + ex.Message);
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
            if (file == null)
            {
                return;
            }

            try
            {
                int count = await SettingsBackupService.ImportAsync(file);
                LoadCloseBehaviorSetting();
                LoadObservedEffectsSettings();
                DisplayScalingSettingsPanel.RefreshSettings();
                ApplyLanguage();
                await ShowSettingsMessageAsync(
                    LocalizationManager.Current == UiLanguage.SimplifiedChinese
                        ? $"已导入 {count} 项设置。重新打开 Game Bar 小组件后全部生效。"
                        : $"Imported {count} settings. Reopen the Game Bar widget to apply all values.");
            }
            catch (Exception ex)
            {
                App.Log("Import settings failed: " + ex);
                await ShowSettingsMessageAsync(
                    (LocalizationManager.Current == UiLanguage.SimplifiedChinese
                        ? "导入设置失败："
                        : "Settings import failed: ") + ex.Message);
            }
        }

        private static async Task ShowSettingsMessageAsync(string message)
        {
            try
            {
                await new MessageDialog(message, "Kill Confirm Overlay").ShowAsync();
            }
            catch
            {
            }
        }

        private void OnHomeNavigationClick(object sender, RoutedEventArgs e)
        {
            SetSettingsPage(true);
        }

        private void OnGameNavigationClick(object sender, RoutedEventArgs e)
        {
            SetSettingsPage(false);
        }

        private void SetSettingsPage(bool showHome)
        {
            _showingHomePage = showHome;
            if (HomePageContent != null)
            {
                HomePageContent.Visibility = showHome ? Visibility.Visible : Visibility.Collapsed;
            }
            if (GamePageContent != null)
            {
                GamePageContent.Visibility = showHome ? Visibility.Collapsed : Visibility.Visible;
            }
            if (DisplayScalingSettingsPanel != null && showHome)
            {
                DisplayScalingSettingsPanel.RefreshSettings();
            }
        }
    }
}
