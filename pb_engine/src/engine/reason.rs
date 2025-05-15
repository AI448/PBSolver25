/// 割り当て理由
#[derive(Clone, Copy)]
pub enum Reason<ExplainKeyT>
where
    ExplainKeyT: Copy,
{
    /// 決定
    Decision,
    /// 伝播
    Propagation {
        // NOTE: 伝播がどの時点で発生したかをここに持たせることも考えられるが， Conflict Analisis で特定したほうが効率的
        /// 伝播を引き起こした制約条件
        explain_key: ExplainKeyT,
    },
}

impl<ExplainKeyT> Reason<ExplainKeyT>
where
    ExplainKeyT: Copy,
{
    #[inline(always)]
    pub fn is_decision(&self) -> bool {
        return matches!(self, Self::Decision);
    }
    #[inline(always)]
    pub fn is_propagation(&self) -> bool {
        return matches!(self, Self::Propagation { .. });
    }
}
