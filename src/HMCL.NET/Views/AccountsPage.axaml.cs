using System;
using System.Threading.Tasks;
using Avalonia.Controls;
using Avalonia.Interactivity;
using HMCL.NET.Util;
using HMCL.NET.ViewModels;
using HMCLCore.NET.Auth;

namespace HMCL.NET.Views;

/// <summary>
/// The accounts page listing saved accounts and allowing offline account creation.
/// Bindings resolve against the owning <see cref="MainWindowViewModel"/> data context.
/// </summary>
public partial class AccountsPage : UserControl
{
    /// <summary>
    /// Creates a new AccountsPage.
    /// </summary>
    public AccountsPage()
    {
        InitializeComponent();
        AddAccountButton.Click += OnAddAccountClick;
        CreateOfflineAccountButton.Click += OnCreateOfflineAccountClick;
        RemoveAccountButton.Click += OnRemoveAccountClick;
        UseAccountMenuItem.Click += OnUseAccountClick;
        CopyAccountInfoMenuItem.Click += OnCopyAccountInfoClick;
        SetSkinMenuItem.Click += OnSetSkinClick;
        ResetSkinMenuItem.Click += OnResetSkinClick;
        PreviewSkinMenuItem.Click += OnPreviewSkinClick;
        RemoveAccountMenuItem.Click += OnRemoveAccountMenuClick;
        LoginAuthServerButton.Click += OnLoginAuthServerClick;
        AddAuthServerButton.Click += OnAddAuthServerClick;
        RemoveAuthServerButton.Click += OnRemoveAuthServerClick;
        AuthServerList.SelectionChanged += OnAuthServerSelectionChanged;
    }

    /// <summary>
    /// Shows the preview of the skin of the selected account.
    /// </summary>
    /// <param name="sender">The menu entry.</param>
    /// <param name="e">The click event.</param>
    private void OnPreviewSkinClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel { SelectedAccount: { } account })
            return;

        if (TopLevel.GetTopLevel(this) is not Window owner)
            return;

        var window = new SkinPreviewWindow(account.Username, account.SkinPath);
        window.ShowDialog(owner);
    }

    /// <summary>
    /// Adds the authlib-injector server entered by the user to the saved
    /// servers.
    /// </summary>
    private async void OnAddAuthServerClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        var url = AuthServerBox.Text?.Trim() ?? string.Empty;
        if (await viewModel.AddAuthServerAsync(url))
            AuthServerBox.Text = string.Empty;
    }

    /// <summary>
    /// Removes the selected authlib-injector server from the saved servers.
    /// </summary>
    private void OnRemoveAuthServerClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel viewModel)
            viewModel.RemoveSelectedAuthServer();
    }

    /// <summary>
    /// Copies the URL of the selected authlib-injector server into the text
    /// box, so that it can be used for the login.
    /// </summary>
    private void OnAuthServerSelectionChanged(object? sender, SelectionChangedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel { SelectedAuthServer: { } server })
            return;

        AuthServerBox.Text = server.Url;
    }

    /// <summary>
    /// Opens the login dialog in Yggdrasil mode with the entered authentication
    /// server URL prefilled, mirroring Java HMCL's authlib-injector login flow.
    /// </summary>
    private async void OnLoginAuthServerClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        var serverUrl = AuthServerBox.Text?.Trim() ?? string.Empty;
        var loginViewModel = new LoginViewModel(viewModel.AccountService)
        {
            ServerUrl = serverUrl
        };

        var dialog = new LoginDialog(loginViewModel);
        dialog.SelectYggdrasil();

        if (TopLevel.GetTopLevel(this) is Window owner)
            await dialog.ShowDialog(owner);

        viewModel.SyncAccounts();
    }

    /// <summary>
    /// Makes the chosen account the account used to launch the game.
    /// </summary>
    /// <param name="sender">The menu item.</param>
    /// <param name="e">The click event.</param>
    private void OnUseAccountClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel viewModel && GetMenuAccount(sender) is { } account)
            viewModel.SelectedAccount = account;
    }

    /// <summary>
    /// Copies the name and the type of the chosen account into the clipboard.
    /// </summary>
    /// <param name="sender">The menu item.</param>
    /// <param name="e">The click event.</param>
    private async void OnCopyAccountInfoClick(object? sender, RoutedEventArgs e)
    {
        if (GetMenuAccount(sender) is not { } account)
            return;

        await ClipboardUtils.SetTextAsync(TopLevel.GetTopLevel(this), account.ToString());
    }

    /// <summary>
    /// Removes the chosen account from the launcher.
    /// </summary>
    /// <param name="sender">The menu item.</param>
    /// <param name="e">The click event.</param>
    private void OnRemoveAccountMenuClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel viewModel && GetMenuAccount(sender) is { } account)
            viewModel.RemoveAccount(account);
    }

    /// <summary>
    /// Asks the user for a skin file and applies it to the chosen account.
    /// </summary>
    /// <param name="sender">The menu item.</param>
    /// <param name="e">The click event.</param>
    private async void OnSetSkinClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        if (GetMenuAccount(sender) is not { } account)
            return;

        if (TopLevel.GetTopLevel(this) is not { } topLevel)
            return;

        var files = await topLevel.StorageProvider.OpenFilePickerAsync(new Avalonia.Platform.Storage.FilePickerOpenOptions
        {
            Title = Lang.Get("hmcl.accountspage.axaml.858_"),
            AllowMultiple = false,
            FileTypeFilter = new[]
            {
                new Avalonia.Platform.Storage.FilePickerFileType(Lang.Get("hmcl.accountspage.axaml.860_")) { Patterns = new[] { "*.png" } }
            }
        });

        if (files.Count == 0 || files[0].Path.Scheme != "file")
            return;

        await viewModel.SetAccountSkinAsync(account, files[0].Path.LocalPath);
    }

    /// <summary>
    /// Removes the skin of the chosen account.
    /// </summary>
    /// <param name="sender">The menu item.</param>
    /// <param name="e">The click event.</param>
    private async void OnResetSkinClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        if (GetMenuAccount(sender) is { } account)
            await viewModel.ResetAccountSkinAsync(account);
    }

    /// <summary>
    /// Returns the account chosen in the context menu, or the selected
    /// account when the menu does not know it.
    /// </summary>
    /// <param name="sender">The menu item showing the account.</param>
    /// <returns>The chosen account, or <c>null</c> when there is none.</returns>
    private Account? GetMenuAccount(object? sender)
    {
        if (sender is MenuItem { DataContext: Account account })
            return account;

        return (DataContext as MainWindowViewModel)?.SelectedAccount;
    }

    /// <summary>
    /// Removes the selected account from the launcher and persists the change.
    /// </summary>
    private void OnRemoveAccountClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        viewModel.RemoveAccount(viewModel.SelectedAccount!);
    }

    /// <summary>
    /// Opens the login dialog and refreshes the account list after it closes.
    /// </summary>
    private async void OnAddAccountClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        var loginViewModel = new LoginViewModel(viewModel.AccountService);
        var dialog = new LoginDialog(loginViewModel);

        if (TopLevel.GetTopLevel(this) is Window owner)
            await dialog.ShowDialog(owner);

        viewModel.SyncAccounts();
    }

    /// <summary>
    /// Adds a new offline account using the username entered in the text box.
    /// </summary>
    private void OnCreateOfflineAccountClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        var username = UsernameBox.Text?.Trim();
        if (string.IsNullOrWhiteSpace(username))
            return;

        viewModel.AddAccount(new OfflineAccount(username));
        UsernameBox.Text = string.Empty;
    }
}
