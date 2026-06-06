using DraoxDemo.Services;
using System.Globalization;
using System.Windows;
using System.Windows.Data;

namespace DraoxDemo.Converters;

/// <summary>
/// Returns HorizontalAlignment.Right if senderId matches current user, otherwise Left.
/// Pass AppState as ConverterParameter.
/// </summary>
public class MessageAlignConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, CultureInfo culture)
    {
        if (value is string senderId && parameter is AppState state)
            return senderId == state.UserId ? HorizontalAlignment.Right : HorizontalAlignment.Left;
        return HorizontalAlignment.Left;
    }

    public object ConvertBack(object value, Type targetType, object parameter, CultureInfo culture)
        => Binding.DoNothing;
}

public class MessageBubbleColorConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, CultureInfo culture)
    {
        if (value is string senderId && parameter is AppState state)
        {
            return senderId == state.UserId
                ? System.Windows.Media.Brushes.DodgerBlue   // own
                : new System.Windows.Media.SolidColorBrush(System.Windows.Media.Color.FromRgb(0x40, 0x42, 0x49)); // others
        }
        return System.Windows.Media.Brushes.Gray;
    }

    public object ConvertBack(object value, Type targetType, object parameter, CultureInfo culture)
        => Binding.DoNothing;
}
