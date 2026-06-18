import { setLang, t, useLang, type Lang } from '@/i18n';
import { Save } from 'lucide-react';
import { useToast } from '@/components/Toast';
import { useEffect, useState } from 'react';
import { usePageTitle } from '@/components/PageTitleContext';

export default function SystemConfig() {
  const { toast } = useToast();
  const { setPageTitle } = usePageTitle();
  const [saving, setSaving] = useState(false);
  const currentLang = useLang();

  useEffect(() => {
    setPageTitle(t('sysConfig.title'), t('sysConfig.subtitle'));
    return () => setPageTitle('');
  }, [setPageTitle, currentLang]);

  const handleLangChange = (lang: string) => {
    setLang(lang as Lang);
  };

  const handleSave = () => {
    setSaving(true);
    setTimeout(() => {
      setSaving(false);
      toast({ title: t('sysConfig.saved'), variant: 'success' });
    }, 500);
  };

  return (
    <div className="space-y-6">
      <div
        className="border rounded-[14px] p-5 flex items-center gap-4"
        style={{ background: 'var(--bg-card)', borderColor: 'var(--border)' }}
      >
        <label className="text-[13px] font-medium shrink-0" style={{ color: 'var(--text-secondary)' }}>
          {t('menu.language')}
        </label>
        <select
          value={currentLang}
          onChange={(e) => handleLangChange(e.target.value)}
          className="w-[180px] px-3 py-2 rounded-[6px] text-[13px]"
          style={{
            background: 'var(--bg-elevated)',
            border: '1px solid var(--border)',
            color: 'var(--text)',
            outline: 'none',
            cursor: 'pointer',
          }}
        >
          <option value="en">English</option>
          <option value="zh">中文</option>
        </select>
      </div>

      <div className="flex justify-end">
        <button
          onClick={handleSave}
          disabled={saving}
          className="flex items-center gap-2 px-5 py-2.5 rounded-[6px] text-[13px] font-semibold text-white disabled:opacity-50"
          style={{
            background: 'linear-gradient(135deg, var(--brand), var(--brand-dark))',
            boxShadow: '0 2px 10px rgba(108,92,231,.3)',
          }}
        >
          <Save size={14} />
          {saving ? t('sysConfig.saving') : t('sysConfig.save')}
        </button>
      </div>
    </div>
  );
}
