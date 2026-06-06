using DraoxDemo.ViewModels;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;

namespace DraoxDemo.Views;

public partial class ChatView : UserControl
{
    public ChatView()
    {
        InitializeComponent();
        DataContextChanged += OnDataContextChanged;
    }

    private void OnDataContextChanged(object sender, DependencyPropertyChangedEventArgs e)
    {
        if (e.OldValue is ChatViewModel oldVm)
            oldVm.Messages.CollectionChanged -= Messages_CollectionChanged;
        if (e.NewValue is ChatViewModel newVm)
            newVm.Messages.CollectionChanged += Messages_CollectionChanged;
    }

    private void Messages_CollectionChanged(object? sender,
        System.Collections.Specialized.NotifyCollectionChangedEventArgs e)
    {
        // Auto-scroll to bottom when new messages arrive
        if (e.Action == System.Collections.Specialized.NotifyCollectionChangedAction.Add)
            Dispatcher.BeginInvoke(() => ScrollView.ScrollToBottom());
    }

    private void MessageBox_KeyDown(object sender, KeyEventArgs e)
    {
        if (e.Key == Key.Enter && !Keyboard.IsKeyDown(Key.LeftShift))
        {
            if (DataContext is ChatViewModel vm && vm.SendCommand.CanExecute(null))
                vm.SendCommand.Execute(null);
            e.Handled = true;
        }
    }

    private void ImageUrl_Click(object sender, RoutedEventArgs e)
    {
        var dialog = new ImageUrlDialog();
        dialog.Owner = Window.GetWindow(this);
        if (dialog.ShowDialog() == true && !string.IsNullOrWhiteSpace(dialog.Url))
        {
            if (DataContext is ChatViewModel vm)
                vm.MessageText = $"[image]{dialog.Url}";
        }
    }
}
