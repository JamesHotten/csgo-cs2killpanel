using KillConfirmGameBar.Services;
using Windows.UI.Xaml;

namespace KillConfirmGameBar
{
    public sealed partial class MainPage
    {
        private void ApplyLanguage()
        {
            TitleText.Text = LocalizationManager.Text("MainTitle");
            bool isChinese = LocalizationManager.Current == UiLanguage.SimplifiedChinese;
            HomeNavigationButton.Content = isChinese ? "主页" : "Home";
            GameNavigationButton.Content = isChinese ? "游戏设置" : "Game settings";
            GameStyleLabelText.Text = isChinese ? "当前游戏模式:" : "Game style:";
            GeneralSettingsTitleText.Text = isChinese ? "通用设置" : "General settings";
            CloseBehaviorLabelText.Text = isChinese ? "关闭主窗口时:" : "When closing:";
            ObservedEffectsLabelText.Text = isChinese ? "观察来源效果:" : "Observed feed effects:";
            ObservedEffectsHintText.Text = isChinese
                ? "三项默认开启，互不影响本地玩家主视角。"
                : "All three are enabled by default and never disable the local-player feed.";
            SpectatedPlayerEffectsToggle.Header = isChinese ? "观战队友" : "Spectated player";
            ReplayEffectsToggle.Header = isChinese ? "回放/演示" : "Replay/demo";
            ControlledBotEffectsToggle.Header = isChinese ? "机器人接管" : "Bot takeover";
            SpectatedPlayerEffectsToggle.OffContent = ReplayEffectsToggle.OffContent =
                ControlledBotEffectsToggle.OffContent = isChinese ? "关" : "Off";
            SpectatedPlayerEffectsToggle.OnContent = ReplayEffectsToggle.OnContent =
                ControlledBotEffectsToggle.OnContent = isChinese ? "开" : "On";
            SettingsBackupLabelText.Text = isChinese ? "设置备份:" : "Settings backup:";
            ExportSettingsButton.Content = isChinese ? "导出设置" : "Export";
            ImportSettingsButton.Content = isChinese ? "导入设置" : "Import";
            SettingsVersionText.Text = isChinese
                ? $"配置版本 {SettingsConfigurationService.CurrentVersion}"
                : $"Configuration v{SettingsConfigurationService.CurrentVersion}";
            DisplayScalingTitleText.Text = isChinese ? "高分辨率显示设置" : "High-resolution display";
            DisplayScalingDescriptionText.Text = isChinese
                ? "提高 2K/4K 屏幕上的控制面板可读性。"
                : "Improve control panel readability on 2K and 4K displays.";
            DisplayScalingSettingsPanel.ApplyLanguage();

            VoiceCollectionsTitleText.Text = LocalizationManager.Text("VoiceCollectionsTitle");
            VoiceCollectionsHintText.Text = LocalizationManager.Text("VoiceCollectionsHint");
            IconCollectionsTitleText.Text = LocalizationManager.Text("IconCollectionsTitle");
            IconCollectionsHintText.Text = LocalizationManager.Text("IconCollectionsHint");

            ImportVoicePackButton.Content = LocalizationManager.Text("ImportVoicePack");
            ImportVoiceZipButton.Content = LocalizationManager.Text("ImportZip");
            CreateVoicePackButton.Content = LocalizationManager.Text("CreateVoicePack");
            ImportIconPackButton.Content = LocalizationManager.Text("ImportIconPack");
            ImportIconZipButton.Content = LocalizationManager.Text("ImportZip");
            CreateIconPackButton.Content = LocalizationManager.Text("CreateIconPack");

            StructureTitleText.Text = LocalizationManager.Text("StructureTitle");
            StructureBodyText.Text = LocalizationManager.Text("StructureBody");
            StructureImportFolderTitleText.Text = LocalizationManager.Text("StructureImportFolderTitle");
            StructureImportFolderBodyText.Text = LocalizationManager.Text("StructureImportFolderBody");
            StructureVoiceSpecTitleText.Text = LocalizationManager.Text("StructureVoiceSpecTitle");
            StructureVoiceSpecBodyText.Text = LocalizationManager.Text("StructureVoiceSpecBody");
            StructureIconSpecTitleText.Text = LocalizationManager.Text("StructureIconSpecTitle");
            StructureIconSpecSummaryText.Text = LocalizationManager.Text("StructureIconSpecSummary");
            StructureIconSpecFullText.Text = LocalizationManager.Text("StructureIconSpecFull");
            UpdateIconSpecToggleText();
            StructureImportZipTitleText.Text = LocalizationManager.Text("StructureImportZipTitle");
            StructureImportZipBodyText.Text = LocalizationManager.Text("StructureImportZipBody");
            StructureCreatorTitleText.Text = LocalizationManager.Text("StructureCreatorTitle");
            StructureCreatorBodyText.Text = LocalizationManager.Text("StructureCreatorBody");
            StructureFileHintText.Text = LocalizationManager.Text("StructureFileHint");

            TipsTitleText.Text = LocalizationManager.Text("TipsTitle");
            TipsBodyText.Text = LocalizationManager.Text("TipsBody");

            ApplyGameStyleUi();
        }

        private void OnIconSpecToggleClick(object sender, RoutedEventArgs e)
        {
            _iconSpecExpanded = !_iconSpecExpanded;
            StructureIconSpecFullText.Visibility = _iconSpecExpanded ? Visibility.Visible : Visibility.Collapsed;
            UpdateIconSpecToggleText();
        }

        private void UpdateIconSpecToggleText()
        {
            if (IconSpecToggleButton == null)
            {
                return;
            }

            IconSpecToggleButton.Content = LocalizationManager.Text(
                _iconSpecExpanded ? "StructureIconSpecCollapse" : "StructureIconSpecExpand");
        }
    }
}
