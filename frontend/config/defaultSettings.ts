import type { ProLayoutProps } from '@ant-design/pro-components';

// Default ProLayout settings with full dark theme token config
const defaultSettings: ProLayoutProps & {
  pwa?: boolean;
  logo?: string;
} = {
  navTheme: 'realDark',
  colorPrimary: '#e05d10',
  layout: 'mix',
  contentWidth: 'Fluid',
  fixedHeader: true,
  fixSiderbar: true,
  colorWeak: false,
  title: 'Draox Admin',
  pwa: false,
  logo: '/logo.svg',
  iconfontUrl: '',
  token: {
    // Sider tokens
    sider: {
      colorMenuBackground: '#16213e',
      colorMenuItemDivider: '#2a2a4a',
      colorTextMenuSelected: '#ff8c42',
      colorTextMenuActive: '#ff8c42',
      colorTextMenuItemHover: '#ff8c42',
      colorTextMenu: '#a0a0b0',
      colorTextMenuSecondary: '#707090',
      colorBgMenuItemSelected: 'rgba(224, 93, 16, 0.15)',
      colorBgMenuItemHover: 'rgba(255, 140, 66, 0.08)',
      colorBgMenuItemCollapsedElevated: '#16213e',
      colorBgCollapsedButton: '#16213e',
      colorTextCollapsedButton: '#a0a0b0',
      colorTextCollapsedButtonHover: '#ff8c42',
      paddingInlineLayoutMenu: 8,
    },
    // Header tokens
    header: {
      colorBgHeader: '#0f3460',
      colorHeaderTitle: '#ff8c42',
      colorTextMenuSelected: '#ff8c42',
      colorTextMenuActive: '#ff8c42',
      colorTextMenu: '#a0a0b0',
      colorTextMenuSecondary: '#707090',
      colorBgMenuItemSelected: 'rgba(224, 93, 16, 0.15)',
      colorBgMenuItemHover: 'rgba(255, 140, 66, 0.08)',
      colorBgRightActionsItemHover: 'rgba(255, 140, 66, 0.08)',
      heightLayoutHeader: 56,
    },
    // Page container tokens
    pageContainer: {
      paddingInlinePageContainerContent: 24,
      paddingBlockPageContainerContent: 24,
      colorBgPageContainer: '#1a1a2e',
      colorBgPageContainerChildren: '#16213e',
    },
    // Global layout tokens
    bgLayout: '#1a1a2e',
    colorTextAppListIconHover: '#ff8c42',
    colorTextAppListIcon: '#a0a0b0',
  },
};

export default defaultSettings;
