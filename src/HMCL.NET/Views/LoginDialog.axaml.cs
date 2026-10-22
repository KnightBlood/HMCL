using System;
using Avalonia.Controls;
using Avalonia.Interactivity;
using HMCL.NET.ViewModels;
using HMCLCore.NET.Auth;
using HMCL.NET.Util;

namespace HMCL.NET.Views;

/// <summary>
/// Modal dialog for creating accounts. Supports offline login and
/// Yggdrasil (third-party authentication server) login.
/// Bindings resolve against a <see cref="LoginViewModel"/> data context.
/// </summary>
public partial class LoginDialog : Window
{
    /// <summary>
    /// Creates a default LoginDialog for the XAML loader; usage should pass a view model.
    /// </summary>
    public LoginDialog() : this(new LoginViewModel(new AccountManager()))
    {
    }

    /// <summary>
    /// Creates a new LoginDialog backed by the provided login view model.
    /// </summary>
    /// <param name="viewModel">The view model performing authentication.</param>
    public LoginDialog(LoginViewModel viewModel)
    {
        DataContext = viewModel ?? throw new ArgumentNullException(nameof(viewModel));
        InitializeComponent();
        LoginButton.Click += OnLoginClick;
        LoginTypeBox.SelectionChanged += OnLoginTypeChanged;
        BrowseButton.Click += (_, _) => (DataContext as LoginViewModel)?.OpenAuthorizationPage();
        SelectCharacterButton.Click += OnSelectCharacterClick;
        OnLoginTypeChanged(this, null!);

        // The escape key closes the dialog and the enter key logs in.
        this.EnableEscapeToClose();
        this.EnableEnterTo(() => OnLoginClick(this, null!));
    }

    /// <summary>
    /// Selects the Yggdrasil (third-party authentication server) login tab.
    /// </summary>
    public void SelectYggdrasil()
    {
        LoginTypeBox.SelectedIndex = 1;
    }

    /// <summary>
    /// Shows or hides the input fields depending on the selected account type.
    /// Offline login needs the username, Yggdrasil adds the server URL and
    /// password, and Microsoft login needs the pasted authorization code.
    /// </summary>
    private void OnLoginTypeChanged(object? sender, SelectionChangedEventArgs e)
    {
        var isYggdrasil = LoginTypeBox.SelectedIndex == 1;
        var isMicrosoft = LoginTypeBox.SelectedIndex == 2;

        PasswordLabel.IsVisible = !isMicrosoft;
        PasswordBox.IsVisible = !isMicrosoft;
        ServerUrlLabel.IsVisible = isYggdrasil;
        ServerUrlBox.IsVisible = isYggdrasil;
        AuthCodeLabel.IsVisible = isMicrosoft;
        AuthCodeBox.IsVisible = isMicrosoft;
        BrowseButton.IsVisible = isMicrosoft;
    }

    /// <summary>
    /// Performs the login according to the selected account type
    /// and closes the dialog once an account has been registered.
    /// </summary>
    private async void OnLoginClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not LoginViewModel viewModel)
            return;

        var enteredLogin = viewModel.IsLoggingIn;
        if (enteredLogin)
            return;

        LoginButton.IsEnabled = false;
        try
        {
            if (LoginTypeBox.SelectedIndex == 1)
            {
                await viewModel.LoginYggdrasilAsync();
            }
            else if (LoginTypeBox.SelectedIndex == 2)
            {
                await viewModel.LoginMicrosoftAsync();
            }
            else
            {
                viewModel.LoginOffline();
            }
        }
        finally
        {
            LoginButton.IsEnabled = true;

            // The character selection is shown when the account owns several
            // characters.
            CharacterPanel.IsVisible = viewModel.HasCharacterSelection;

            // Close only when a fresh account was actually registered.
            if (viewModel.LastLoginAccount != null)
                Close();
        }
    }

    /// <summary>
    /// Uses the character chosen by the user and closes the dialog when the
    /// account is registered.
    /// </summary>
    /// <param name="sender">The button.</param>
    /// <param name="e">The click event.</param>
    private async void OnSelectCharacterClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not LoginViewModel viewModel)
            return;

        await viewModel.SelectCharacterAsync();
        CharacterPanel.IsVisible = viewModel.HasCharacterSelection;

        if (viewModel.LastLoginAccount != null)
            Close();
    }
}
