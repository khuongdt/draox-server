using System.Windows;

namespace DraoxDemo.Views;

public partial class ImageUrlDialog : Window
{
    public string Url => UrlBox.Text.Trim();

    public ImageUrlDialog()
    {
        InitializeComponent();
        UrlBox.Focus();
    }

    private void Insert_Click(object sender, RoutedEventArgs e)
    {
        if (!string.IsNullOrWhiteSpace(UrlBox.Text))
            DialogResult = true;
    }

    private void Cancel_Click(object sender, RoutedEventArgs e)
    {
        DialogResult = false;
    }
}
