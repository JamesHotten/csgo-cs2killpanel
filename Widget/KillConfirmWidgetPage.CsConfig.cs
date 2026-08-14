using System;
using System.Linq;
using System.Threading.Tasks;
using KillConfirmGameBar.Services;
using Windows.Data.Json;
using Windows.Storage;
using Windows.Storage.AccessCache;
using Windows.Storage.Pickers;
using Windows.Storage.Streams;
using Windows.UI.Popups;
using Windows.UI.Xaml;
using Windows.UI.Xaml.Controls;
using Windows.Web.Http;

namespace KillConfirmGameBar
{
    public sealed partial class KillConfirmWidgetPage
    {
        private async void OnSelectCsFolderClick(object sender, RoutedEventArgs e)
        {
            var picker = new FolderPicker
            {
                SuggestedStartLocation = PickerLocationId.ComputerFolder,
                ViewMode = PickerViewMode.List
            };
            picker.FileTypeFilter.Add("*");

            StorageFolder folder = await picker.PickSingleFolderAsync();
            if (folder == null)
            {
                return;
            }

            try
            {
                SaveCsFolder(folder);
                await RefreshCfgStatusAsync();
            }
            catch (Exception ex)
            {
                App.Log("Failed to save selected CS folder: " + ex);
                UpdateCfgStatus(CfgDetectionState.Error, LocalizationManager.Text("CfgFolderError"), LocalizationManager.Text("CfgFolderSaveError"));
            }
        }

        private async void OnInstallCfgClick(object sender, RoutedEventArgs e)
        {
            if (_csInstallFolder == null && string.IsNullOrWhiteSpace(_serviceDetectedCsRootPath))
            {
                await ShowCfgMessageAsync(LocalizationManager.Text("SelectCsFirst"));
                return;
            }

            var dialog = new MessageDialog(
                LocalizationManager.Text("AddCfgQuestion"),
                LocalizationManager.Text("AddCfgTitle"));
            string addText = LocalizationManager.Text("Add");
            dialog.Commands.Add(new UICommand(addText));
            dialog.Commands.Add(new UICommand(LocalizationManager.Text("Cancel")));
            dialog.DefaultCommandIndex = 0;
            dialog.CancelCommandIndex = 1;

            IUICommand result = await dialog.ShowAsync();
            if (result.Label != addText)
            {
                return;
            }

            await InstallCfgAsync();
        }

        private async Task LoadSavedCsFolderAsync()
        {
            string token = ApplicationData.Current.LocalSettings.Values[CsInstallFolderTokenSettingKey] as string;
            if (string.IsNullOrWhiteSpace(token))
            {
                await TryAutoDetectCsFolderAsync();
                return;
            }

            try
            {
                _csInstallFolder = await StorageApplicationPermissions.FutureAccessList.GetFolderAsync(token);
                await RefreshCfgStatusAsync();
            }
            catch (Exception ex)
            {
                App.Log("Failed to restore CS folder access: " + ex);
                _csInstallFolder = null;
                await TryAutoDetectCsFolderAsync();
            }
        }

        private async Task TryAutoDetectCsFolderAsync()
        {
            if (_csInstallFolder != null)
            {
                return;
            }

            UpdateCfgStatus(CfgDetectionState.Checking, LocalizationManager.Text("CfgAutoDetecting"), LocalizationManager.Text("CfgSelectRootHint"));

            try
            {
                await EnsureServiceAvailableAsync();

                using (var client = await LocalServiceAuth.CreateHttpClientAsync())
                using (HttpResponseMessage response = await client.GetAsync(Cs2RootUri))
                {
                    if (!response.IsSuccessStatusCode)
                    {
                        UpdateCfgStatus(CfgDetectionState.NotSelected, null, LocalizationManager.Text("CfgSelectRootHint"));
                        return;
                    }

                    string responseText = await response.Content.ReadAsStringAsync();
                    JsonObject json = JsonObject.Parse(responseText);
                    ApplyDetectedInstallations(json);
                    bool found = json.GetNamedBoolean("found", false);
                    string path = _detectedCsInstallations.Count > 0
                        ? _serviceDetectedCsRootPath
                        : json.GetNamedString("path", string.Empty);
                    string cfgStatus = _detectedCsInstallations.Count > 0
                        ? _serviceDetectedCfgStatus
                        : json.GetNamedString("cfg_status", string.Empty);

                    if (!found || string.IsNullOrWhiteSpace(path))
                    {
                        UpdateCfgStatus(CfgDetectionState.NotSelected, null, LocalizationManager.Text("CfgSelectRootHint"));
                        return;
                    }

                    _serviceDetectedCsRootPath = path;
                    _serviceDetectedCfgStatus = cfgStatus;

                    try
                    {
                        StorageFolder folder = await StorageFolder.GetFolderFromPathAsync(path);
                        SaveCsFolder(folder);
                        await RefreshCfgStatusAsync();
                    }
                    catch (Exception ex)
                    {
                        App.Log("Auto-detected CS folder, but folder access failed: " + ex);
                        ApplicationData.Current.LocalSettings.Values[CsInstallFolderPathSettingKey] = path;
                        ApplyServiceDetectedCfgStatus(path, cfgStatus);
                    }
                }
            }
            catch (Exception ex)
            {
                App.Log("Failed to auto-detect CS folder: " + ex);
                UpdateCfgStatus(CfgDetectionState.NotSelected, null, LocalizationManager.Text("CfgSelectRootHint"));
            }
        }

        private void SaveCsFolder(StorageFolder folder)
        {
            StorageApplicationPermissions.FutureAccessList.AddOrReplace(CsInstallFolderAccessToken, folder);
            ApplicationData.Current.LocalSettings.Values[CsInstallFolderTokenSettingKey] = CsInstallFolderAccessToken;
            ApplicationData.Current.LocalSettings.Values[CsInstallFolderPathSettingKey] = folder.Path;
            _csInstallFolder = folder;
        }

        private async Task RefreshCfgStatusAsync()
        {
            if (_csInstallFolder == null)
            {
                UpdateCfgStatus(CfgDetectionState.NotSelected, null, LocalizationManager.Text("CfgSelectRootHint"));
                return;
            }

            UpdateCfgStatus(CfgDetectionState.Checking, null, GetCsFolderDisplayText());

            StorageFolder cfgFolder = await TryGetCfgFolderAsync(_csInstallFolder);
            if (cfgFolder == null)
            {
                UpdateCfgStatus(CfgDetectionState.Error, null, LocalizationManager.Text("CfgWrongFolderHint"));
                return;
            }

            try
            {
                StorageFile cfgFile = await cfgFolder.GetFileAsync(GsiConfigFileName);
                string configText = await FileIO.ReadTextAsync(cfgFile);
                string[] requiredReplayKeys =
                {
                    "\"bomb\"",
                    "\"player_position\"",
                    "\"allplayers_id\"",
                    "\"allplayers_state\"",
                    "\"allplayers_weapons\"",
                    "\"allplayers_match_stats\""
                };
                if (requiredReplayKeys.Any(key =>
                    configText.IndexOf(key, StringComparison.OrdinalIgnoreCase) < 0))
                {
                    await FileIO.WriteTextAsync(cfgFile, GsiConfigText, UnicodeEncoding.Utf8);
                }

                UpdateCfgStatus(CfgDetectionState.Ready, null, GetCsFolderDisplayText());
            }
            catch (System.IO.FileNotFoundException)
            {
                UpdateCfgStatus(CfgDetectionState.Missing, null, GetCsFolderDisplayText());
            }
            catch (Exception ex)
            {
                App.Log("Failed to check cfg file: " + ex);
                UpdateCfgStatus(CfgDetectionState.Error, null, GetCsFolderDisplayText());
            }
        }

        private async Task InstallCfgAsync()
        {
            if (_csInstallFolder == null && !string.IsNullOrWhiteSpace(_serviceDetectedCsRootPath))
            {
                await InstallCfgThroughServiceAsync();
                return;
            }

            try
            {
                UpdateCfgStatus(CfgDetectionState.Checking, LocalizationManager.Text("CfgAdding"), GetCsFolderDisplayText());
                StorageFolder cfgFolder = await GetOrCreateCfgFolderAsync(_csInstallFolder);
                StorageFile cfgFile = await cfgFolder.CreateFileAsync(GsiConfigFileName, CreationCollisionOption.ReplaceExisting);
                await FileIO.WriteTextAsync(cfgFile, GsiConfigText, UnicodeEncoding.Utf8);
                UpdateCfgStatus(CfgDetectionState.Ready, null, GetCsFolderDisplayText());
            }
            catch (Exception ex)
            {
                App.Log("Failed to install cfg file: " + ex);
                UpdateCfgStatus(CfgDetectionState.Error, LocalizationManager.Text("CfgAddFailed"), GetCsFolderDisplayText());
                await ShowCfgMessageAsync(LocalizationManager.Text("CfgWriteFailed"));
            }
        }

        private string GetCsFolderDisplayText()
        {
            string savedPath = ApplicationData.Current.LocalSettings.Values[CsInstallFolderPathSettingKey] as string;
            if (!string.IsNullOrWhiteSpace(savedPath))
            {
                return savedPath;
            }

            return _csInstallFolder?.Path ?? _csInstallFolder?.Name ?? "Counter-Strike Global Offensive";
        }

        private static async Task<StorageFolder> TryGetCfgFolderAsync(StorageFolder root)
        {
            if (root == null)
            {
                return null;
            }
            if (string.Equals(root.Name, "cfg", StringComparison.OrdinalIgnoreCase))
            {
                return root;
            }

            StorageFolder csgoFolder = await TryResolveCsgoFolderAsync(root, 0);
            return await TryGetChildFolderAsync(csgoFolder, "cfg");
        }

        private async Task InstallCfgThroughServiceAsync()
        {
            try
            {
                UpdateCfgStatus(CfgDetectionState.Checking, LocalizationManager.Text("CfgAdding"), _serviceDetectedCsRootPath);
                string requestUri = CounterStrikeCfgUri.AbsoluteUri
                    + "?path=" + Uri.EscapeDataString(_serviceDetectedCsRootPath);
                if (!string.IsNullOrWhiteSpace(_serviceDetectedCsVersion))
                {
                    requestUri += "&version=" + Uri.EscapeDataString(_serviceDetectedCsVersion);
                }
                using (var client = await LocalServiceAuth.CreateHttpClientAsync())
                using (HttpResponseMessage response = await client.PostAsync(new Uri(requestUri), null))
                {
                    if (!response.IsSuccessStatusCode)
                    {
                        throw new InvalidOperationException("CFG service install failed: " + response.StatusCode);
                    }

                    string responseText = await response.Content.ReadAsStringAsync();
                    JsonObject json = JsonObject.Parse(responseText);
                    ApplyDetectedInstallations(json);
                    _serviceDetectedCfgStatus = json.GetNamedString("cfg_status", "ready");
                    ApplyServiceDetectedCfgStatus(_serviceDetectedCsRootPath, _serviceDetectedCfgStatus);
                }
            }
            catch (Exception ex)
            {
                App.Log("Failed to install cfg through local service: " + ex);
                UpdateCfgStatus(CfgDetectionState.Error, LocalizationManager.Text("CfgAddFailed"), _serviceDetectedCsRootPath);
                await ShowCfgMessageAsync(LocalizationManager.Text("CfgWriteFailed"));
            }
        }

        private void ApplyServiceDetectedCfgStatus(string path, string cfgStatus)
        {
            switch ((cfgStatus ?? string.Empty).Trim().ToLowerInvariant())
            {
                case "ready":
                    UpdateCfgStatus(CfgDetectionState.Ready, null, path);
                    break;
                case "missing":
                case "outdated":
                    UpdateCfgStatus(CfgDetectionState.Missing, null, path);
                    break;
                default:
                    UpdateCfgStatus(CfgDetectionState.NotSelected, null, LocalizationManager.Text("CfgDetectedNeedConfirm") + path);
                    break;
            }
        }

        private void ApplyDetectedInstallations(JsonObject json)
        {
            if (CfgInstallationsSelector == null)
            {
                return;
            }

            string preferredPath = _serviceDetectedCsRootPath;
            _detectedCsInstallations.Clear();
            JsonArray installations = json.GetNamedArray("installations", new JsonArray());
            foreach (IJsonValue value in installations)
            {
                if (value.ValueType != JsonValueType.Object)
                {
                    continue;
                }

                JsonObject entry = value.GetObject();
                string path = entry.GetNamedString("path", string.Empty).Trim();
                if (string.IsNullOrWhiteSpace(path))
                {
                    continue;
                }

                _detectedCsInstallations.Add(new DetectedCsInstallation
                {
                    Path = path,
                    Version = entry.GetNamedString("version", "cs2"),
                    CfgStatus = entry.GetNamedString("cfg_status", "not_found")
                });
            }

            _suppressCfgInstallationSelectionEvents = true;
            try
            {
                CfgInstallationsSelector.Items.Clear();
                int selectedIndex = 0;
                for (int index = 0; index < _detectedCsInstallations.Count; index++)
                {
                    DetectedCsInstallation installation = _detectedCsInstallations[index];
                    CfgInstallationsSelector.Items.Add(new ComboBoxItem
                    {
                        Content = installation.DisplayText,
                        Tag = installation
                    });
                    if (!string.IsNullOrWhiteSpace(preferredPath)
                        && string.Equals(
                            preferredPath,
                            installation.Path,
                            StringComparison.OrdinalIgnoreCase))
                    {
                        selectedIndex = index;
                    }
                }

                CfgInstallationsSelector.Visibility = _detectedCsInstallations.Count > 0
                    ? Visibility.Visible
                    : Visibility.Collapsed;
                if (_detectedCsInstallations.Count > 0)
                {
                    CfgInstallationsSelector.SelectedIndex = selectedIndex;
                    SelectDetectedInstallation(_detectedCsInstallations[selectedIndex]);
                }
            }
            finally
            {
                _suppressCfgInstallationSelectionEvents = false;
            }
        }

        private void OnCfgInstallationSelectionChanged(object sender, SelectionChangedEventArgs e)
        {
            if (_suppressCfgInstallationSelectionEvents)
            {
                return;
            }

            if (CfgInstallationsSelector.SelectedItem is ComboBoxItem item
                && item.Tag is DetectedCsInstallation installation)
            {
                _csInstallFolder = null;
                SelectDetectedInstallation(installation);
                ApplyServiceDetectedCfgStatus(installation.Path, installation.CfgStatus);
            }
        }

        private void SelectDetectedInstallation(DetectedCsInstallation installation)
        {
            _serviceDetectedCsRootPath = installation.Path;
            _serviceDetectedCsVersion = installation.Version;
            _serviceDetectedCfgStatus = installation.CfgStatus;
            ApplicationData.Current.LocalSettings.Values[CsInstallFolderPathSettingKey] =
                installation.Path;
        }

        private static async Task<StorageFolder> TryGetChildFolderAsync(StorageFolder parent, string name)
        {
            if (parent == null)
            {
                return null;
            }

            try
            {
                return await parent.GetFolderAsync(name);
            }
            catch
            {
                return null;
            }
        }

        private static async Task<StorageFolder> GetOrCreateCfgFolderAsync(StorageFolder root)
        {
            if (root != null && string.Equals(root.Name, "cfg", StringComparison.OrdinalIgnoreCase))
            {
                return root;
            }

            StorageFolder csgoFolder = await TryResolveCsgoFolderAsync(root, 0);
            if (csgoFolder == null)
            {
                throw new InvalidOperationException("The selected folder does not contain a CS2 or CS:GO Legacy installation.");
            }
            return await csgoFolder.CreateFolderAsync("cfg", CreationCollisionOption.OpenIfExists);
        }

        private static async Task<StorageFolder> TryResolveCsgoFolderAsync(StorageFolder folder, int depth)
        {
            if (folder == null || depth > 10)
            {
                return null;
            }
            if (string.Equals(folder.Name, "csgo", StringComparison.OrdinalIgnoreCase))
            {
                return folder;
            }

            StorageFolder game = await TryGetChildFolderAsync(folder, "game");
            StorageFolder cs2Csgo = await TryGetChildFolderAsync(game, "csgo");
            if (cs2Csgo != null)
            {
                return cs2Csgo;
            }

            StorageFolder legacyCsgo = await TryGetChildFolderAsync(folder, "csgo");
            if (legacyCsgo != null)
            {
                return legacyCsgo;
            }

            string[] spine = { "steam", "steamapps", "common", "Counter-Strike Global Offensive", "csgo legacy" };
            foreach (string name in spine)
            {
                StorageFolder child = await TryGetChildFolderAsync(folder, name);
                StorageFolder result = await TryResolveCsgoFolderAsync(child, depth + 1);
                if (result != null)
                {
                    return result;
                }
            }

            if (string.Equals(folder.Name, "common", StringComparison.OrdinalIgnoreCase))
            {
                try
                {
                    int checkedFolders = 0;
                    foreach (StorageFolder child in await folder.GetFoldersAsync())
                    {
                        StorageFolder result = await TryResolveCsgoFolderAsync(child, depth + 1);
                        if (result != null)
                        {
                            return result;
                        }
                        if (++checkedFolders >= 200)
                        {
                            break;
                        }
                    }
                }
                catch
                {
                }
            }

            return null;
        }

        private async Task ShowCfgMessageAsync(string message)
        {
            try
            {
                await new MessageDialog(message, LocalizationManager.Text("CfgMessageTitle")).ShowAsync();
            }
            catch
            {
            }
        }
    }
}
