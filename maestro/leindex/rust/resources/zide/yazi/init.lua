-- auto-layout logic (consolidated for stability)
function Tab:layout()
    local area = self._area
    if not area then return end
    
    local r = MANAGER.ratio
    if not r then return end

    if area.w > 80 then
        self._chunks = ui.Layout()
            :direction(ui.Layout.HORIZONTAL)
            :constraints({
                ui.Constraint.Ratio(r.parent, r.all),
                ui.Constraint.Ratio(r.current, r.all),
                ui.Constraint.Ratio(r.preview, r.all)
            })
            :split(area)
    elseif area.w > 40 then
        self._chunks = ui.Layout()
            :direction(ui.Layout.HORIZONTAL)
            :constraints({
                ui.Constraint.Ratio(0, r.all),
                ui.Constraint.Ratio(r.current + r.parent, r.all),
                ui.Constraint.Ratio(r.preview + r.parent, r.all)
            })
            :split(area)
    else
        self._chunks = ui.Layout()
            :direction(ui.Layout.HORIZONTAL)
            :constraints({
                ui.Constraint.Ratio(0, r.all),
                ui.Constraint.Ratio(r.all, r.all),
                ui.Constraint.Ratio(0, r.all),
            })
            :split(area)
    end
end
