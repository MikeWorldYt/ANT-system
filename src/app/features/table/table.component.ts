import { CommonModule } from '@angular/common';
import { Component, Input } from '@angular/core';
import { content } from '../../docs/content/content';
import { LanguageService } from '../../services/navLanguage.service';
import { Subscription } from 'rxjs';

@Component({
  selector: 'table-it',
  standalone: true,
  imports: [
    CommonModule,
  ],
  templateUrl: './table.component.html',
  styleUrl: './table.component.css'
})
export class TableComponent {
  @Input() path!: [string, string, string];
  @Input() article!: string;
  @Input() write!: string;

  constructor(
    private languageService: LanguageService,
  ) { }

  tableHead: any[] = [];
  tableContent: any[] = [];

  ngOnInit(): void {
    this.article = this.concatArticle(this.article);
    this.subscribeToLanguageChanges();
    this.resolveContent();
  }

  languageSubscription!: Subscription;
  private subscribeToLanguageChanges(): void {
    this.languageSubscription = this.languageService.currentLanguage$.subscribe((Lang) => {
      this.path[0] = Lang;
      this.resolveContent();
    });
  }

  private resolveContent(): void {
    const [lang, title, page] = this.path;
    const article = this.article;
    const tableData = content[lang][title][page][article][this.write];

    this.tableHead = tableData.table1_head;
    this.tableContent = tableData.table1_content;
  }




  private concatArticle(article: string): string {
    return `article_${article}`;
  }

}
