import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'OpenTrace',
  description: '基于 eBPF 的 Linux 内核可观测工具',
  base: '/opentrace/',

  head: [
    ['link', { rel: 'icon', href: '/logo.png' }],
  ],

  themeConfig: {
    logo: '/logo.png',

    nav: [
      { text: '指南', link: '/guide/introduction' },
      { text: '开发', link: '/guide/development/overview' },
      { text: '示例', link: '/guide/examples/exec-tracepoint' },
      {
        text: '相关链接',
        items: [
          { text: 'GitHub', link: 'https://github.com/3Xpl0it3r/opentrace' },
          { text: 'Issues', link: 'https://github.com/3Xpl0it3r/opentrace/issues' },
        ]
      }
    ],

    sidebar: {
      '/guide/': [
        {
          text: '开始',
          items: [
            { text: '简介', link: '/guide/introduction' },
            { text: '快速开始', link: '/guide/quickstart' },
            { text: '环境要求', link: '/guide/requirements' },
          ]
        },
        {
          text: '使用',
          items: [
            { text: 'CLI 命令', link: '/guide/cli' },
            { text: 'MCP 服务', link: '/guide/mcp' },
          ]
        },
        {
          text: '开发',
          items: [
            { text: '开发概览', link: '/guide/development/overview' },
            { text: '协议扩展', link: '/guide/development/protocol-extension' },
          ]
        },
        {
          text: '示例',
          items: [
            { text: 'Exec Tracepoint', link: '/guide/examples/exec-tracepoint' },
          ]
        }
      ]
    },

    socialLinks: [
      { icon: 'github', link: 'https://github.com/3Xpl0it3r/opentrace' }
    ],

    footer: {
      message: '基于 Apache-2.0 许可发布',
      copyright: 'Copyright © 2024-present OpenTrace Contributors'
    },

    search: {
      provider: 'local',
      options: {
        translations: {
          button: {
            buttonText: '搜索文档',
            buttonAriaLabel: '搜索文档'
          },
          modal: {
            noResultsText: '无法找到相关结果',
            resetButtonTitle: '清除查询条件',
            footer: {
              selectText: '选择',
              navigateText: '切换',
              closeText: '关闭'
            }
          }
        }
      }
    },

    outline: {
      label: '页面导航'
    },

    lastUpdated: {
      text: '最后更新于'
    },

    docFooter: {
      prev: '上一页',
      next: '下一页'
    }
  }
})
