#pragma once
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QSaveFile>
#include <QtCore/QFileDevice>
#include <QtGui/QFontMetricsF>
#include <QtGui/QPainter>
#include <QtGui/QPdfWriter>
#include <QtGui/QPageSize>
#include <algorithm>

namespace bermuda_pdf {
// A4 coordinates in points. Qt embeds fonts and draws charts/boards as vectors.
class ReportPainter {
    QPdfWriter &pdf;
    QPainter &p;
    qreal y = 36;
    int page = 1;
    const qreal width = 523;
    QString title;
    QFont font(qreal size, bool bold = false) {
        QFont f; f.setPointSizeF(size); f.setBold(bold); return f;
    }
    void footer() {
        p.setFont(font(8)); p.setPen(QColor("#52616a"));
        p.drawText(QRectF(36, 802, width, 20), Qt::AlignLeft,
            QFontMetricsF(p.font()).elidedText(title, Qt::ElideRight, width-45));
        p.drawText(QRectF(36, 802, width, 20), Qt::AlignRight, QString::number(page));
    }
public:
    bool ok = true;
    ReportPainter(QPdfWriter &writer, QPainter &painter, const QString &name)
        : pdf(writer), p(painter), title(name) {}
    void room(qreal height) {
        if (y+height <= 780) return;
        footer();
        if (!pdf.newPage()) { ok=false; return; }
        ++page; y=36;
    }
    void text(const QString &value, qreal size=10, bool bold=false) {
        p.setFont(font(size,bold));
        QFontMetricsF metrics(p.font());
        // Break very long text across pages instead of clipping names/comments.
        QStringList lines;
        for (const QString &paragraph : value.split('\n')) {
            QString line;
            for (const QChar ch : paragraph) {
                if (!line.isEmpty() && metrics.horizontalAdvance(line+ch)>width) {
                    lines.append(line); line.clear();
                }
                line+=ch;
            }
            lines.append(line);
        }
        for (const QString &line : lines) {
            room(metrics.height()+2);
            p.setFont(font(size,bold)); p.setPen(QColor("#202e35"));
            p.drawText(QPointF(36,y+metrics.ascent()),line);
            y+=metrics.height()+2;
        }
        y+=5;
    }
    void coverage(const QJsonArray &counts, double total) {
        room(55);
        const QStringList labels={"Complete","Partial","Awaiting analysis","Unidentified"};
        const QColor colours[]={QColor("#32877d"),QColor("#6b93ae"),QColor("#b58a45"),QColor("#9c9c9c")};
        p.fillRect(QRectF(36,y,width,15),QColor("#edf0f2"));
        qreal x=36;
        for (int i=0;i<4;i++) {
            const qreal w=total>0 ? counts[i].toDouble()/total*width : 0;
            p.fillRect(QRectF(x,y,w,15),colours[i]);x+=w;
        }
        y+=24;
        QStringList legend;
        for (int i=0;i<4;i++) legend.append(QString("%1 %2").arg(counts[i].toDouble(),0,'f',0).arg(labels[i]));
        text(legend.join(" · "),9);
    }
    void losses(const QJsonArray &counts, double total) {
        if (total<=0) { text("No personal moves assessed in this analysis yet."); return; }
        room(128);
        const QStringList labels={"Below 1 point","1–3 points","3–5 points","5+ points"};
        const QColor colours[]={QColor("#32877d"),QColor("#6b93ae"),QColor("#b58a45"),QColor("#ab6262")};
        p.setFont(font(9));
        for (int i=0;i<4;i++) {
            const double pct=100*counts[i].toDouble()/total;
            p.setPen(QColor("#202e35"));
            p.drawText(QPointF(36,y+11),labels[i]);
            p.fillRect(QRectF(125,y,280,14),QColor("#edf0f2"));
            p.fillRect(QRectF(125,y,std::clamp(pct,0.0,100.0)*2.8,14),colours[i]);
            p.drawText(QPointF(414,y+11),QString("%1% · %2 moves").arg(pct,0,'f',1).arg(counts[i].toDouble(),0,'f',0));
            y+=25;
        }
        for (int pct : {0,50,100}) p.drawText(QPointF(125+pct*2.8-6,y+4),QString("%1%").arg(pct));
        y+=20;
    }
    void board(const QJsonObject &card) {
        const int size=card["size"].toInt();
        if (size<2 || size>52) { text("Board size unavailable."); return; }
        room(244);
        const qreal left=52, top=y+14, side=210, step=side/(size-1);
        p.fillRect(QRectF(36,y,242,242),QColor("#fbf5e8"));
        p.setFont(font(6));
        for (int i=0;i<size;i++) {
            p.setPen(QPen(QColor("#8f887b"),0.4));
            p.drawLine(QPointF(left,top+i*step),QPointF(left+side,top+i*step));
            p.drawLine(QPointF(left+i*step,top),QPointF(left+i*step,top+side));
            p.setPen(QColor("#202e35"));
            const QString label=QString(QChar('A'+i+(i>=8 ? 1 : 0)));
            p.drawText(QRectF(left+i*step-6,y,12,10),Qt::AlignCenter,label);
            p.drawText(QRectF(36,top+i*step-5,14,10),Qt::AlignCenter,QString::number(size-i));
        }
        for (const auto value : card["stones"].toArray()) {
            const auto stone=value.toArray();
            p.setPen(QPen(QColor("#222222"),0.45));
            p.setBrush(stone[2].toBool() ? QColor("#222222") : QColor("white"));
            p.drawEllipse(QPointF(left+stone[0].toInt()*step,top+stone[1].toInt()*step),step*0.45,step*0.45);
        }
        const auto played=card["played"].toArray();
        if (played.size()==2) {
            p.setBrush(Qt::NoBrush);p.setPen(QPen(QColor("#ba3e45"),1.5));
            p.drawEllipse(QPointF(left+played[0].toInt()*step,top+played[1].toInt()*step),step*0.36,step*0.36);
        }
        p.setBrush(Qt::NoBrush);p.setFont(font(10));p.setPen(QColor("#202e35"));
        QString info=QString("Before move %1\n\nYou played %2\n\nWin estimate: %3% → %4%\n\nEstimated cost: %5 points")
            .arg(card["move"].toInt()).arg(card["played_name"].toString())
            .arg(card["win_before"].toDouble(),0,'f',1).arg(card["win_after"].toDouble(),0,'f',1)
            .arg(card["loss"].toDouble(),0,'f',1);
        if (card["already_unlikely"].toBool()) info+="\n\nAlready ≤5% before this move";
        info+="\n\nRed ring: recorded move.";
        p.drawText(QRectF(298,y+10,250,225),Qt::TextWordWrap,info);
        y+=254;
    }
    void render(const QJsonObject &data) {
        text(data["heading"].toString(),20,true);
        text(data["subtitle"].toString(),12,true);
        text("Saved "+data["saved"].toString(),9);
        text("Preserved analysis summary. Later analysis does not change this report.",9);
        if (data["unfinished"].toBool()) text("Unfinished period when saved.",10,true);
        const double total=data["total"].toDouble();
        text(QString("Analysis coverage · %1 games").arg(total,0,'f',0),13,true);
        coverage(data["coverage"].toArray(),total);
        if (total==0) text("No games in this period. This is a gap in play, not a zero move-quality score.");
        const auto groups=data["groups"].toArray();
        if (groups.isEmpty()) text("No saved analysis to chart in this report.");
        for (const auto value : groups) {
            const auto group=value.toObject();
            room(200);
            text(group["heading"].toString(),13,true);
            text(group["description"].toString(),9);
            losses(group["buckets"].toArray(),group["moves"].toDouble());
            text("Share of assessed personal moves, including partial games. Below 1 includes estimated gains. This is not a strength grade.",9);
            const auto cards=group["cards"].toArray();
            if (cards.isEmpty()) { text("No saved decisions above the report thresholds.");continue; }
            room(320);
            text("Positions to investigate",13,true);
            text(group["by_win"].toBool() ?
                "Largest saved win-estimate drops, with one position per game (up to three)." :
                "Examples of estimated point losses; no large win-estimate drops were saved.",9);
            for (const auto c : cards) {
                const auto card=c.toObject();
                room(310);
                text(card["title"].toString(),11,true);
                text(card["context"].toString(),9);
                board(card);
            }
        }
        footer();
    }
};

// Empty string means success. QSaveFile leaves any existing destination intact
// if painting or writing fails. No shell commands or external browser required.
inline QString save_report_pdf(const QString &json, const QString &filename) {
    QJsonParseError error;
    const auto document=QJsonDocument::fromJson(json.toUtf8(),&error);
    if (error.error!=QJsonParseError::NoError || !document.isObject())
        return QStringLiteral("Unable to read report presentation data.");
    QSaveFile file(filename);
    if (!file.open(QIODevice::WriteOnly)) return file.errorString();
    bool successful=false;
    {
        QPdfWriter writer(&file);
        writer.setResolution(72);
        writer.setPageSize(QPageSize(QPageSize::A4));
        writer.setPageMargins(QMarginsF(0,0,0,0),QPageLayout::Point);
        writer.setTitle(document.object()["title"].toString());
        writer.setCreator(QStringLiteral("Bermuda"));
        QPainter painter;
        if (!painter.begin(&writer)) return QStringLiteral("Unable to start PDF rendering.");
        painter.setRenderHint(QPainter::Antialiasing);
        ReportPainter report(writer,painter,document.object()["title"].toString());
        report.render(document.object());
        successful=painter.end() && report.ok;
    }
    if (!successful || file.error()!=QFileDevice::NoError) {
        file.cancelWriting();
        return QStringLiteral("Unable to finish writing the PDF.");
    }
    if (!file.commit()) return file.errorString();
    return {};
}
}
