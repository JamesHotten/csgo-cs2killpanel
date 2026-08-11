using KillConfirmGameBar.Services;
using Windows.UI.Xaml;
using Windows.UI.Xaml.Controls;

namespace KillConfirmGameBar.Controls.Settings
{
    public sealed partial class ValorantAdvancedSettingsPanel : UserControl
    {
        private bool _suppressStreakEvents;

        public ValorantAdvancedSettingsPanel()
        {
            InitializeComponent();
            _suppressStreakEvents = true;
            SharedStreakSettingsPanelSupport.Load(GameStyleMode.Valorant, StreakModeSelector);
            AssistAudioToggle.IsOn = AssistAudioSettingsStore.Load(GameStyleMode.Valorant);
            _suppressStreakEvents = false;
        }

        internal void ApplyTheme(GameThemePalette theme)
        {
            SettingsPanelSupport.ApplyPanel(Card, TitleText, BodyText, theme);
            SettingsPanelSupport.ApplySettingRow(StreakModeLabel, StreakModeSelector, theme);
            SettingsPanelSupport.ApplyToggleRow(AssistAudioLabel, AssistAudioToggle, theme);
        }

        public void ApplyLanguage(bool isChinese)
        {
            TitleText.Text = isChinese ? "VAL 高级设置" : "VAL advanced settings";
            BodyText.Text = string.Empty;
            BodyText.Visibility = Windows.UI.Xaml.Visibility.Collapsed;
            SharedStreakSettingsStore.ApplyLanguage(
                StreakModeLabel,
                StreakLifeItem,
                StreakTimed5Item,
                StreakTimed10Item,
                StreakTimed15Item,
                isChinese);
            AssistAudioLabel.Text = isChinese ? "\u52a9\u653b\u97f3\u6548" : "Assist audio";
            AssistAudioToggle.OnContent = isChinese ? "\u6709\u58f0\u97f3\uff08common\uff09" : "Sound (common)";
            AssistAudioToggle.OffContent = isChinese ? "\u65e0\u58f0\u97f3\uff08\u9ed8\u8ba4\uff09" : "Muted (default)";
        }

        private async void OnStreakModeSelectionChanged(object sender, SelectionChangedEventArgs e)
        {
            if (!_suppressStreakEvents)
            {
                await SharedStreakSettingsPanelSupport.SaveAndSyncAsync(
                    GameStyleMode.Valorant,
                    StreakModeSelector,
                    AssistAudioToggle.IsOn,
                    true);
            }
        }

        private async void OnAssistAudioToggled(object sender, RoutedEventArgs e)
        {
            if (_suppressStreakEvents)
            {
                return;
            }

            AssistAudioSettingsStore.Save(GameStyleMode.Valorant, AssistAudioToggle.IsOn);
            await SharedStreakSettingsPanelSupport.SaveAndSyncAsync(
                GameStyleMode.Valorant,
                StreakModeSelector,
                AssistAudioToggle.IsOn,
                true);
        }
    }
}
