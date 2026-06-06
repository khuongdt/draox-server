using System.Windows;

namespace DraoxDemo.Views;

public partial class EditMessageDialog : Window
{
    public string NewText { get; private set; } = string.Empty;

    public EditMessageDialog(string currentText)
    {
        InitializeComponent();
        TextEdit.Text = currentText;
        TextEdit.SelectAll();
        TextEdit.Focus();
    }

    private void Save_Click(object sender, RoutedEventArgs e)
    {
        NewText = TextEdit.Text.Trim();
        if (string.IsNullOrEmpty(NewText)) return;
        DialogResult = true;
    }

    private void Cancel_Click(object sender, RoutedEventArgs e)
    {
        DialogResult = false;
    }
}
