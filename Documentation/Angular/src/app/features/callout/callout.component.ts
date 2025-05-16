import { Component, Input } from '@angular/core';
import { CommonModule } from '@angular/common';
import { content } from '../../docs/content/content';
import { LanguageService } from '../../services/navLanguage.service';
import { Subscription } from 'rxjs';
import { AnchorComponent } from '../anchor/anchor.component';

@Component({
  selector: 'callout',
  standalone: true,
  imports: [
    CommonModule,
    AnchorComponent,
  ],
  templateUrl: './callout.component.html',
  styleUrl: './callout.component.css'
})
export class CalloutComponent {
  @Input() path!: [string, string, string];
  @Input() article!: string;
  @Input() write!: string;

  constructor(
    private languageService: LanguageService,
  ) { }

  writer: string = '';
  type: string = '';
  items: { key: string, value: any }[] = [];
  articleInherit: string = '';
  writeInherit: string = '';

  ngOnInit(): void {
    this.articleInherit = this.article;
    this.writeInherit = this.write;
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
    const write = this.write;
    const langDEF = "EN";
    // Resolve Logic
    this.writer = content[lang][title][page][article][write];
    this.type = content[langDEF][title][page][article][write].t;
    // Filter and return in "items"
    this.items = Object.entries(this.writer)
      .filter(([key]) => key.startsWith('c_') || key.startsWith('a_'))
      .map(([key, value]) => ({ key, value }));
  }

  private concatArticle(article: string): string {
    return `article_${article}`;
  }

}