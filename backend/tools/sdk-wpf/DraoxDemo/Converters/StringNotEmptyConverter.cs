using System.Globalization;
using System.Windows;
using System.Windows.Data;

namespace DraoxDemo.Converters;

[ValueConversion(typeof(string), typeof(Visibility))]
public class StringNotEmptyToVisibilityConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, CultureInfo culture)
        => !string.IsNullOrWhiteSpace(value as string) ? Visibility.Visible : Visibility.Collapsed;

    public object ConvertBack(object value, Type targetType, object parameter, CultureInfo culture)
        => Binding.DoNothing;
}
